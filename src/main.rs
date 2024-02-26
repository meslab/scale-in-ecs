mod autoscaling;
mod ecs;
mod elasticache;
mod rds;

use clap::Parser;
use log::{debug, info};
use tokio;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[clap(
    version = "v0.0.2",
    author = "Anton Sidorov tonysidrock@gmail.com",
    about = "Scale down ECS cluster"
)]
struct Args {
    #[clap(short, long, default_value = "direc-prod-lb")]
    cluster: String,

    #[clap(short, long, default_value = "eu-central-1")]
    region: String,

    #[clap(short, long, default_value = "false")]
    delete: bool,

    #[clap(short, long, default_value = "false")]
    scaledown: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args = Args::parse();

    let as_client = autoscaling::initialize_client(&args.region).await;
    let asgs = autoscaling::list_asgs(&as_client, &args.cluster, 0).await?;
    info!("ASGs: {:?}", asgs);

    let elc_client = elasticache::initialize_client(&args.region).await;
    let replication_groups =
        elasticache::list_replication_groups(&elc_client, &args.cluster).await?;
    info!("Replication Groups: {:?}", replication_groups);

    let ecs_client = ecs::initialize_client(&args.region).await;
    let services = ecs::get_service_arns(&ecs_client, &args.cluster, 0).await?;
    info!("Services: {:?}", services);

    let rds_client = rds::initialize_client(&args.region).await;
    let db_instances = rds::list_db_instances(&rds_client, &args.cluster).await?;
    info!("DB Instances: {:?}", db_instances);

    if args.scaledown || args.delete {
        for asg in &asgs {
            autoscaling::scale_down_asg(&as_client, &asg, 0).await?;
        }
        for service in &services {
            ecs::scale_down_service(&ecs_client, &args.cluster, &service, 0).await?;
        }
        for db_instance in &db_instances {
            rds::stop_db_instance(&rds_client, &db_instance).await?;
        }
    }

    if args.delete {
        for replication_group in replication_groups {
            elasticache::delete_replication_group(&elc_client, &replication_group).await?;
        }
        for service in &services {
            ecs::delete_service(&ecs_client, &args.cluster, &service).await?;
        }
        for db_instance in &db_instances {
            rds::disable_deletion_protection(&rds_client, &db_instance).await?;
            rds::delete_db_instance(&rds_client, &db_instance).await?;
        }
    }

    debug!("Cluster: {} Region: {}.", &args.cluster, &args.region);

    Ok(())
}
