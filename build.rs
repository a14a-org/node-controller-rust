fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Compile the protocol buffer definitions
    tonic_build::compile_protos("proto/node_service.proto")?;
    
    println!("cargo:rerun-if-changed=proto/node_service.proto");
    
    Ok(())
} 