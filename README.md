# HopsFS Object Store
An [object_store](https://docs.rs/object_store/latest/object_store/) implementation for HopsFS/HDFS using libhdfs Go client bindings.

This is a fork of [hdfs-native-object-store](https://github.com/datafusion-contrib/hdfs-native-object-store) modified to use HopsFS Go client libraries instead of the native Rust hdfs-native library.

# Compatibility
|hopsfs-object-store|object_store|
|---|---|
|1.0.x|>=0.12.2, <0.13|

# Usage
```rust
use hdfs_native_object_store::HdfsObjectStore;
let store = HdfsObjectStore::with_url("hdfs://localhost:8020")?;
```

# Documentation
See [Documentation](https://docs.rs/hdfs-native-object-store).
