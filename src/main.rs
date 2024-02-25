use aws_config::default_provider::credentials::DefaultCredentialsChain;
use aws_sdk_autoscaling::config::Region as ASRegion;
use aws_sdk_autoscaling::{Client as ASClient, Config as ASConfig};
use aws_sdk_ecs::config::Region as EcsRegion;
use aws_sdk_ecs::{Client as EcsClient, Config as EcsConfig};
use clap::Parser;
use log::{debug, info};
use tokio;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[clap(
    version = "v0.0.1",
    author = "Anton Sidorov tonysidrock@gmail.com",
    about = "Counts wwords frequency in a text file"
)]
struct Args {
    #[clap(short, long, default_value = "direc-prod-lb")]
    cluster: String,

    #[clap(short, long, default_value = "eu-central-1")]
    region: String,
}

async fn get_service_arn(
    ecs_client: &EcsClient,
    cluster: &String,
    service: &String,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut ecs_services_stream = ecs_client
        .list_services()
        .cluster(cluster)
        .max_results(100)
        .into_paginator()
        .send();

    while let Some(services) = ecs_services_stream.next().await {
        debug!("Services: {:?}", services);
        let service_arn = services
            .unwrap()
            .service_arns
            .unwrap()
            .into_iter()
            .find(|arn| arn.contains(service));
        if let Some(service_arn) = service_arn {
            debug!("Inside get_service_arn Service ARN: {:?}", service_arn);
            return Ok(service_arn);
        }
    }
    Err("Service not found".into())
}

async fn list_asgs(
    as_client: &ASClient,
    cluster: &String,
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
                .auto_scaling_group_name.clone()
                .unwrap()
                .contains(cluster)
                {
                asgs.push(group.auto_scaling_group_name.unwrap());
            }
        }
    }
    Ok(asgs)
}

async fn scale_down_asg(
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args = Args::parse();

    let as_region = ASRegion::new(args.region.clone());
    let as_credentials_provider = DefaultCredentialsChain::builder()
        .region(as_region.clone())
        .build()
        .await;
    let as_config = ASConfig::builder()
        .credentials_provider(as_credentials_provider)
        .region(as_region)
        .build();
    let as_client = ASClient::from_conf(as_config);

    let asgs = list_asgs(&as_client, &args.cluster).await?;
    info!("ASGs: {:?}", asgs);

    let ecs_region = EcsRegion::new(args.region.clone());
    let ecs_credentials_provider = DefaultCredentialsChain::builder()
        .region(ecs_region.clone())
        .build()
        .await;
    let ecs_config = EcsConfig::builder()
        .credentials_provider(ecs_credentials_provider)
        .region(ecs_region)
        .build();

    debug!("Cluster: {} Region: {}.", &args.cluster, &args.region);

    let ecs_client = EcsClient::from_conf(ecs_config);

    Ok(())
}
