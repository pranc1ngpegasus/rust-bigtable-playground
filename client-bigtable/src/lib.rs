use gcloud_sdk::google::bigtable::v2::bigtable_client::BigtableClient;
use gcloud_sdk::google::bigtable::v2::mutate_rows_request::Entry;
use gcloud_sdk::google::bigtable::v2::mutation::SetCell;
use gcloud_sdk::google::bigtable::v2::row_range::EndKey;
use gcloud_sdk::google::bigtable::v2::row_range::StartKey;
use gcloud_sdk::google::bigtable::v2::MutateRowsRequest;
use gcloud_sdk::google::bigtable::v2::Mutation;
use gcloud_sdk::google::bigtable::v2::ReadRowsRequest;
use gcloud_sdk::google::bigtable::v2::RowRange;
use gcloud_sdk::google::bigtable::v2::RowSet;
use gcloud_sdk::{GoogleApi, GoogleAuthMiddleware};
use std::future::Future;
use std::pin::Pin;

pub trait Bigtable: Send + Sync + 'static {
    fn mutate_row<'a>(
        &'a mut self,
        input: MutateRowsInput,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>>;

    fn read_rows<'a>(
        &'a mut self,
        input: ReadRowsInput,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<ReadRowsOutput>> + Send + 'a>>;
}

#[derive(Debug)]
pub struct MutateRowInputMutation {
    pub family_name: String,
    pub column_qualifier: String,
    pub value: String,
}

#[derive(Debug)]
pub struct MutateRowInputEntry {
    pub row_key: String,
    pub mutations: Vec<MutateRowInputMutation>,
}

#[derive(Debug)]
pub struct MutateRowsInput {
    pub table_name: String,
    pub row_key: String,
    pub entries: Vec<MutateRowInputEntry>,
}

#[derive(Debug)]
pub struct ReadRowsInput {
    pub table_name: String,
    pub row_key: String,
}

#[derive(Debug)]
pub struct ReadRowsEntry {
    pub row_key: String,
    pub value: String,
}

#[derive(Debug)]
pub struct ReadRowsOutput {
    pub entries: Vec<ReadRowsEntry>,
}

#[derive(Clone)]
pub struct BigtableImpl {
    client: BigtableClient<GoogleAuthMiddleware>,
}

impl BigtableImpl {
    pub fn try_new() -> Pin<Box<dyn Future<Output = anyhow::Result<Self>> + Send>> {
        Box::pin(async move {
            let client = GoogleApi::from_function(
                BigtableClient::new,
                "https://bigtable.googleapis.com",
                None,
            )
            .await
            .map_err(|e| anyhow::anyhow!("failed to initialize bigtable client: {}", e))?
            .get();

            Ok(Self { client })
        })
    }
}

impl Bigtable for BigtableImpl {
    fn mutate_row<'a>(
        &'a mut self,
        input: MutateRowsInput,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let entries: Vec<Entry> = input
                .entries
                .iter()
                .map(|entry| Entry {
                    row_key: entry.row_key.as_bytes().to_vec(),
                    mutations: entry
                        .mutations
                        .iter()
                        .map(|input| Mutation {
                            mutation: Some(
                                gcloud_sdk::google::bigtable::v2::mutation::Mutation::SetCell(
                                    SetCell {
                                        family_name: input.family_name.clone(),
                                        column_qualifier: input
                                            .column_qualifier
                                            .as_bytes()
                                            .to_vec(),
                                        timestamp_micros: -1,
                                        value: input.value.as_bytes().to_vec(),
                                    },
                                ),
                            ),
                        })
                        .collect(),
                })
                .collect();

            let req = MutateRowsRequest {
                table_name: input.table_name,
                authorized_view_name: String::new(),
                app_profile_id: String::from("default"),
                entries,
            };

            match &self.client.mutate_rows(req).await {
                Ok(_) => Ok(()),
                Err(e) => anyhow::bail!("failed to mutate rows: {}", e),
            }
        })
    }

    fn read_rows<'a>(
        &'a mut self,
        input: ReadRowsInput,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<ReadRowsOutput>> + Send + 'a>> {
        Box::pin(async move {
            let rows: RowSet = RowSet {
                row_keys: Vec::new(),
                row_ranges: vec![RowRange {
                    start_key: Some(StartKey::StartKeyOpen(input.row_key.as_bytes().to_vec())),
                    end_key: Some(EndKey::EndKeyOpen(input.row_key.as_bytes().to_vec())),
                }],
            };

            let req = ReadRowsRequest {
                table_name: input.table_name,
                authorized_view_name: String::new(),
                app_profile_id: String::from("default"),
                rows: Some(rows),
                filter: None,
                rows_limit: 0,
                request_stats_view: 0,
                reversed: false,
            };

            match self.client.read_rows(req).await {
                Ok(mut resp) => {
                    let entries: Vec<ReadRowsEntry> = match resp
                        .get_mut()
                        .message()
                        .await
                        .map_err(|e| anyhow::anyhow!("failed to read response: {}", e))?
                    {
                        Some(read_rows_response) => read_rows_response
                            .chunks
                            .iter()
                            .filter_map(|chunk| {
                                let Ok(row_key) = String::from_utf8(chunk.row_key.clone()) else {
                                    return None;
                                };

                                let Ok(value) = String::from_utf8(chunk.value.clone()) else {
                                    return None;
                                };

                                Some(ReadRowsEntry { row_key, value })
                            })
                            .collect(),
                        None => Vec::new(),
                    };

                    Ok(ReadRowsOutput { entries })
                }
                Err(e) => anyhow::bail!("failed to read rows: {}", e),
            }
        })
    }
}
