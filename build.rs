use std::io::Result;
fn main() -> Result<()> {
    let proto_files = &[
        "src/protos/gogoproto/gogo.proto",
        "src/protos/types.proto",
        "src/protos/remote.proto",
    ];
    let proto_includes = &["src/protos"];
    prost_build::compile_protos(proto_files, proto_includes)?;
    Ok(())
}
