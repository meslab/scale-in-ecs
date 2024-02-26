use aws_config::default_provider::credentials::DefaultCredentialsChain;
use aws_sdk_elasticache::config::Region as ElcRegion;
use aws_sdk_elasticache::{Client as ElcClient, Config as ElcConfig};
use log::debug;


pub async fn initialize_elc_client(region: &str) -> ElcClient {
    let elc_region = ElcRegion::new(region.to_owned());
    let elc_credentials_provider = DefaultCredentialsChain::builder()
        .region(elc_region.clone())
        .build()
        .await;
    let elc_config = ElcConfig::builder()
        .credentials_provider(elc_credentials_provider)
        .region(elc_region)
        .build();
    ElcClient::from_conf(elc_config)
}

pub async fn list_replication_groups(
    elc_client: &ElcClient,
    cluster: &String,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut replication_groups = Vec::new();
    let mut replication_groups_stream = elc_client
        .describe_replication_groups()
        .max_records(100)
        .into_paginator()
        .send();

    while let Some(replication_group) = replication_groups_stream.next().await {
        debug!("Replication Groups: {:?}", replication_group);
        for group in replication_group.unwrap().replication_groups.unwrap() {
            if group
                .replication_group_id
                .clone()
                .unwrap()
                .contains(cluster)
                && group.status.clone().unwrap().contains("available")
            {
                replication_groups.push(group.replication_group_id.unwrap());
            }
        }
    }
    Ok(replication_groups)
}

pub async fn delete_replication_group(
    elc_client: &ElcClient,
    replication_group_id: &String,
) -> Result<(), Box<dyn std::error::Error>> {
    elc_client
        .delete_replication_group()
        .replication_group_id(replication_group_id)
        .send()
        .await?;
    Ok(())
}

