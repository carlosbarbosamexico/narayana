//! Binary for downloading vision models from command line

use narayana_eye::config::VisionConfig;
use narayana_eye::models::ModelManager;
use narayana_eye::error::VisionError;
use std::sync::Arc;
use std::env;

#[tokio::main]
async fn main() -> Result<(), VisionError> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage: download_model <model_name>");
        eprintln!("Available models: yolo, sam, clip");
        std::process::exit(1);
    }
    
    let model_name = args[1].to_lowercase();
    let config = VisionConfig::default();
    let manager = ModelManager::new(Arc::new(config));
    
    match model_name.as_str() {
        "yolo" => {
            println!("Downloading YOLO model...");
            let path = manager.get_yolo_model().await?;
            println!("YOLO model downloaded to: {:?}", path);
        }
        "sam" => {
            println!("Downloading SAM model...");
            let path = manager.get_sam_model().await?;
            println!("SAM model downloaded to: {:?}", path);
        }
        "clip" => {
            println!("Downloading CLIP model...");
            let path = manager.get_clip_model().await?;
            println!("CLIP model downloaded to: {:?}", path);
        }
        _ => {
            eprintln!("Unknown model: {}", model_name);
            eprintln!("Available models: yolo, sam, clip");
            std::process::exit(1);
        }
    }
    
    Ok(())
}


