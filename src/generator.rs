use anyhow::Result;
use atomic_lib::{urls, Resource, Store, Storelike};
use convert_case::{Case, Casing};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tera::{Context, Tera, Value};

use crate::config::Config;

#[derive(Debug, Serialize)]
struct CrateData {
    name: String,
    version: String,
    description: String,
    ontologies: Vec<OntologyData>,
}

#[derive(Debug, Serialize)]
struct OntologyData {
    name: String,
    module_name: String,
    description: String,
    classes: Vec<ClassData>,
    properties: Vec<PropertyData>,
}

#[derive(Debug, Serialize)]
struct ClassData {
    name: String,
    description: String,
    shortname: String,
    subject: String,
    properties: Vec<PropertyData>,
}

#[derive(Debug, Serialize)]
struct PropertyData {
    name: String,
    shortname: String,
    description: String,
    type_name: String,
    subject: String,
    required: bool,
}

pub struct OntologyGenerator {
    store: Store,
    config: Config,
    tera: Tera,
}

impl OntologyGenerator {
    pub fn new(store: Store, config: Config) -> Result<Self> {
        let mut tera = Tera::default();

        // Add custom filters
        tera.register_filter("snake_case", |value: &Value, _: &HashMap<String, Value>| {
            let string = match value.as_str() {
                Some(s) => s,
                None => return Err("Value is not a string".into()),
            };
            Ok(Value::String(string.to_case(Case::Snake)))
        });

        // Load templates
        tera.add_template_file("templates/crate/Cargo.toml.tera", Some("Cargo.toml"))?;
        tera.add_template_file("templates/crate/src/lib.rs.tera", Some("lib.rs"))?;
        tera.add_template_file("templates/crate/src/ontology.rs.tera", Some("ontology.rs"))?;
        tera.add_template_file("templates/crate/src/mod.rs.tera", Some("mod.rs"))?;
        tera.add_template_file("templates/crate/README.md.tera", Some("README.md"))?;

        Ok(Self {
            store,
            config,
            tera,
        })
    }

    pub fn generate(&self) -> Result<()> {
        // Create output directory structure
        let output_dir = PathBuf::from(&self.config.output_folder);
        fs::create_dir_all(&output_dir)?;
        fs::create_dir_all(output_dir.join("src"))?;

        // Generate crate data
        let crate_data = self.generate_crate_data()?;

        // Generate Cargo.toml
        let cargo_toml = self
            .tera
            .render("Cargo.toml", &Context::from_serialize(&crate_data)?)?;
        fs::write(output_dir.join("Cargo.toml"), cargo_toml)?;

        // Generate README.md
        let readme = self
            .tera
            .render("README.md", &Context::from_serialize(&crate_data)?)?;
        fs::write(output_dir.join("README.md"), readme)?;

        // Generate lib.rs
        let lib_rs = self
            .tera
            .render("lib.rs", &Context::from_serialize(&crate_data)?)?;
        fs::write(output_dir.join("src/lib.rs"), lib_rs)?;

        // Generate ontology modules
        for ontology in &crate_data.ontologies {
            let ontology_code = self
                .tera
                .render("ontology.rs", &Context::from_serialize(ontology)?)?;
            fs::write(
                output_dir
                    .join("src")
                    .join(format!("{}.rs", ontology.module_name)),
                ontology_code,
            )?;
        }

        // Generate mod.rs
        let mod_rs = self
            .tera
            .render("mod.rs", &Context::from_serialize(&crate_data)?)?;
        fs::write(output_dir.join("src/mod.rs"), mod_rs)?;

        Ok(())
    }

    fn generate_crate_data(&self) -> Result<CrateData> {
        let mut ontologies = Vec::new();
        let mut crate_name = String::new();

        // Get the first ontology to derive the crate name
        if let Some(first_ontology_url) = self.config.ontologies.first() {
            let ontology = self.store.get_resource(first_ontology_url)?;
            let name = ontology
                .get_propvals()
                .get(urls::SHORTNAME)
                .map(|v| v.to_string())
                .unwrap_or_else(|| "unknown".to_string());

            // Create crate name: atomic_ontology_{name}
            crate_name = format!(
                "atomic_ontology_{}",
                self.sanitize_name(&name).to_lowercase()
            );
        } else {
            return Err(anyhow::anyhow!("No ontologies specified in config"));
        }

        // Generate data for all ontologies
        for ontology_url in &self.config.ontologies {
            let ontology = self.store.get_resource(ontology_url)?;
            ontologies.push(self.generate_ontology_data(&ontology)?);
        }

        Ok(CrateData {
            name: crate_name,
            version: "0.1.0".to_string(),
            description: format!(
                "Generated Atomic Data types for {}",
                ontologies
                    .first()
                    .map(|o| &o.name)
                    .unwrap_or(&"unknown".to_string())
            ),
            ontologies,
        })
    }

    fn generate_ontology_data(&self, ontology: &Resource) -> Result<OntologyData> {
        let name = ontology
            .get_propvals()
            .get(urls::SHORTNAME)
            .map(|v| v.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let description = ontology
            .get_propvals()
            .get(urls::DESCRIPTION)
            .map(|v| v.to_string())
            .unwrap_or_else(|| "".to_string());

        let module_name = self.sanitize_name(&name).to_case(Case::Snake);

        let mut classes = Vec::new();
        let mut properties = Vec::new();

        // Get classes
        if let Some(class_list) = ontology.get_propvals().get(urls::CLASSES) {
            for class_subject in class_list.to_string().split(',').filter(|s| !s.is_empty()) {
                let class = self.store.get_resource(class_subject)?;
                classes.push(self.generate_class_data(&class)?);
            }
        }

        // Get properties
        if let Some(prop_list) = ontology.get_propvals().get(urls::PROPERTIES) {
            for prop_subject in prop_list.to_string().split(',').filter(|s| !s.is_empty()) {
                let prop = self.store.get_resource(prop_subject)?;
                properties.push(self.generate_property_data(&prop)?);
            }
        }

        Ok(OntologyData {
            name,
            module_name,
            description,
            classes,
            properties,
        })
    }

    fn generate_class_data(&self, class: &Resource) -> Result<ClassData> {
        let shortname = class
            .get_propvals()
            .get(urls::SHORTNAME)
            .map(|v| v.to_string())
            .ok_or_else(|| anyhow::anyhow!("Class missing shortname"))?;

        let name = self.sanitize_name(&shortname).to_case(Case::Pascal);

        let description = class
            .get_propvals()
            .get(urls::DESCRIPTION)
            .map(|v| v.to_string())
            .unwrap_or_else(|| "".to_string());

        let mut properties = Vec::new();

        if let Some(required_props) = class.get_propvals().get(urls::REQUIRES) {
            for prop_subject in required_props
                .to_string()
                .split(',')
                .filter(|s| !s.is_empty())
            {
                let prop = self.store.get_resource(prop_subject)?;
                let mut prop_data = self.generate_property_data(&prop)?;
                prop_data.required = true;
                properties.push(prop_data);
            }
        }

        Ok(ClassData {
            name,
            shortname,
            description,
            subject: class.get_subject().to_string(),
            properties,
        })
    }

    fn generate_property_data(&self, property: &Resource) -> Result<PropertyData> {
        let shortname = property
            .get_propvals()
            .get(urls::SHORTNAME)
            .map(|v| v.to_string())
            .ok_or_else(|| anyhow::anyhow!("Property missing shortname"))?;

        let name = self.sanitize_name(&shortname).to_case(Case::Snake);

        let description = property
            .get_propvals()
            .get(urls::DESCRIPTION)
            .map(|v| v.to_string())
            .unwrap_or_else(|| "".to_string());

        let type_name = self.atomic_type_to_rust_type(property)?;

        Ok(PropertyData {
            name,
            shortname,
            description,
            type_name,
            subject: property.get_subject().to_string(),
            required: false,
        })
    }

    fn atomic_type_to_rust_type(&self, property: &Resource) -> Result<String> {
        let datatype = property
            .get_propvals()
            .get(urls::DATATYPE_PROP)
            .map(|v| v.to_string())
            .ok_or_else(|| anyhow::anyhow!("Property missing datatype"))?;

        Ok(match datatype.as_str() {
            urls::STRING => "String".into(),
            urls::INTEGER => "i64".into(),
            urls::BOOLEAN => "bool".into(),
            urls::RESOURCE_ARRAY => "Vec<String>".into(),
            urls::ATOMIC_URL => "String".into(),
            _ => "String".into(),
        })
    }

    fn sanitize_name(&self, name: &str) -> String {
        name.replace('-', "_")
    }
}
