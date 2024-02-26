use aws_config::default_provider::credentials::DefaultCredentialsChain;
use aws_sdk_autoscaling::config::Region as ASRegion;
use aws_sdk_autoscaling::{Client as ASClient, Config as ASConfig};
use log::debug;

pub async fn initialize_as_client(region: &str) -> ASClient {
    let as_region = ASRegion::new(region.to_owned());
    let as_credentials_provider = DefaultCredentialsChain::builder()
        .region(as_region.clone())
        .build()
        .await;
    let as_config = ASConfig::builder()
        .credentials_provider(as_credentials_provider)
        .region(as_region)
        .build();
    ASClient::from_conf(as_config)
}

pub async fn list_asgs(
    as_client: &ASClient,
    cluster: &String,
    desired_capacity: i32,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut asgs = Vec::new();
    let mut asg_stream = as_client
        .describe_auto_scaling_groups()
        .max_records(100)
        .into_paginator()
        .send();

    while let Some(asg) = asg_stream.next().await {
        debug!("ASG: {:?}", asg);
        for group in asg.unwrap().auto_scaling_groups.unwrap() {
            if group
                .auto_scaling_group_name
                .clone()
                .unwrap()
                .contains(cluster)
                && group
                    .desired_capacity
                    .clone()
                    .unwrap()
                    .gt(&desired_capacity)
            {
                asgs.push(group.auto_scaling_group_name.unwrap());
            }
        }
    }
    Ok(asgs)
}

pub async fn scale_down_asg(
    as_client: &ASClient,
    asg_name: &String,
    desired_capacity: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    as_client
        .update_auto_scaling_group()
        .auto_scaling_group_name(asg_name)
        .desired_capacity(desired_capacity)
        .min_size(desired_capacity)
        .max_size(desired_capacity)
        .send()
        .await?;
    Ok(())
}
