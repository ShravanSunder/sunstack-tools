use specta::{
    Types,
    datatype::{DataType, Field, Fields},
};
use std::error::Error;

/// Preserve exclusive single-key Serde alternatives in structural TypeScript unions.
pub(super) fn exclude_other_fields(
    types: &mut Types,
    type_name: &str,
    module: &str,
) -> Result<(), Box<dyn Error>> {
    let mut outcome: Result<(), String> = Err(format!("{type_name} is not registered"));
    types.iter_mut(|named| {
        if named.name != type_name || !named.module_path.ends_with(module) {
            return;
        }
        outcome = (|| {
            let Some(DataType::Enum(enumeration)) = &mut named.ty else {
                return Err(format!("{type_name} is not an enum"));
            };
            let mut keys = Vec::new();
            for (_, variant) in &enumeration.variants {
                let Fields::Named(fields) = &variant.fields else {
                    return Err(format!("{type_name} variant is not named"));
                };
                let [(key, field)] = fields.fields.as_slice() else {
                    return Err(format!("{type_name} variant must have one field"));
                };
                if field.optional || field.ty.is_none() {
                    return Err(format!("{type_name} value is not required"));
                }
                keys.push(key.clone());
            }
            for (_, variant) in &mut enumeration.variants {
                if let Fields::Named(fields) = &mut variant.fields {
                    for key in &keys {
                        if !fields.fields.iter().any(|(name, _)| name == key) {
                            let mut excluded =
                                Field::new(DataType::Reference(specta_typescript::define("never")));
                            excluded.optional = true;
                            fields.fields.push((key.clone(), excluded));
                        }
                    }
                }
            }
            Ok(())
        })();
    });
    outcome.map_err(Into::into)
}
