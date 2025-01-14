use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Compile messages.proto using protoc
    prost_build::compile_protos(&["proto/messages.proto"], &["proto/"])?;
    Ok(())
}
