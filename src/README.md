# Atomic Ontology Generator

Generate Rust types from Atomic Data ontologies.

## Usage

1. Create a configuration file `atomic.config.json`:

```json
{
  "outputFolder": "./src/ontologies",
  "ontologies": ["https://atomicdata.dev/ontologies/core"]
}
```

2. Run the generator:

```bash
cargo run -- --config atomic.config.json
```

3. Use the generated types in your code:

``rust
use atomic_lib::{Store, Storelike};
use your_crate::ontologies::;
fn main() -> anyhow::Result<()> {
let store = Store::init()?;
// Get a resource
let resource = store.get_resource("https://example.com/resource")?;
// Convert to typed struct
let typed = YourType::from_resource(&resource, &store)?;
// Access typed properties
println!("Name: {}", typed.name);
Ok(())
}

````

## Testing

The generated code includes tests for each type. Run them with:

```bash
cargo test
````

## Features

- Generates Rust structs from Atomic Data ontologies
- Implements serialization/deserialization
- Includes type conversion from Atomic Resources
- Generates tests for each type
- Includes example usage
