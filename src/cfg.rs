use crate::Error;

use super::types::{ColumnType, InnerColumnType, OperationType, Setting};
use serde_json::{Number, Value};

/// Parse a value against the schema's column type
fn validate_value(
    v: Value,
    column_type: &ColumnType,
    column_id: &str,
    nullable: bool,
) -> Result<Value, Error> {
    if v == Value::Null {
        if !nullable {
            return Err(format!(
                "Validation error in column {}, expected non-nullable value but got null",
                column_id
            )
            .into());
        } else {
            return Ok(Value::Null);
        }
    }

    match &column_type {
        ColumnType::Scalar { inner } => {
            // Special case: JSON columns can be any type
            if matches!(v, Value::Array(_)) && !matches!(inner, InnerColumnType::Json { .. }) {
                return Err(format!(
                    "Validation error in column {}, expected scalar but got array",
                    column_id
                )
                .into());
            }

            match inner {
                InnerColumnType::String {
                    min_length,
                    max_length,
                    allowed_values,
                    ..
                } => match v {
                    Value::String(s) => {
                        if let Some(min_length) = min_length {
                            if s.len() < *min_length {
                                return Err(format!(
                                    "Validation error in column {}, expected String with min length {} but got String with length {}",
                                    column_id, min_length, s.len()
                                )
                                .into());
                            }
                        }

                        if let Some(max_length) = max_length {
                            if s.len() > *max_length {
                                return Err(format!(
                                    "Validation error in column {}, expected String with max length {} but got String with length {}",
                                    column_id, max_length, s.len()
                                )
                                .into());
                            }
                        }

                        if !allowed_values.is_empty() && !allowed_values.contains(&s) {
                            return Err(format!(
                                "Validation error in column {}, expected String with value in {:?} but got String with value {}",
                                column_id, allowed_values, s
                            )
                            .into());
                        }

                        Ok(Value::String(s))
                    }
                    _ => Err(format!(
                        "Validation error in column {}, expected String but got {:?}",
                        column_id, v
                    )
                    .into()),
                },
                InnerColumnType::Integer {} => match v {
                    Value::String(s) => {
                        if s.is_empty() {
                            Err(format!(
                                "Validation error in column {}, expected Integer but got empty String",
                                column_id
                            ).into())
                        } else {
                            let value = match s.parse::<i64>() {
                                Ok(v) => v,
                                Err(e) => {
                                    return Err(format!(
                                        "Validation error in column {}, expected Integer but got String that cannot be parsed: {}",
                                        column_id, e
                                    )
                                    .into());
                                }
                            };

                            Ok(Value::Number(value.into()))
                        }
                    }
                    Value::Number(v) => {
                        if v.is_i64() {
                            Ok(Value::Number(v))
                        } else {
                            Err(format!(
                                "Validation error in column {}, expected Integer but got Float",
                                column_id
                            )
                            .into())
                        }
                    }
                    _ => Err(format!(
                        "Validation error in column {}, expected Integer but got {:?}",
                        column_id, v
                    )
                    .into()),
                },
                InnerColumnType::Float {} => match v {
                    Value::String(s) => {
                        let value = match s.parse::<f64>() {
                            Ok(v) => v,
                            Err(e) => {
                                return Err(format!(
                                    "Validation error in column {}, expected Float but got String that cannot be parsed: {}",
                                    column_id, e
                                )
                                .into());
                            }
                        };

                        let number = match Number::from_f64(value) {
                            Some(n) => n,
                            None => {
                                return Err(format!(
                                    "Validation error in column {}, expected Float but got Float that cannot be converted to JSON Number",
                                    column_id
                                )
                                .into());
                            }
                        };

                        Ok(Value::Number(number))
                    }
                    Value::Number(v) => {
                        if v.is_f64() {
                            Ok(Value::Number(v))
                        } else {
                            Err(format!(
                                "Validation error in column {}, expected Float but got Integer",
                                column_id
                            )
                            .into())
                        }
                    }
                    _ => Err(format!(
                        "Validation error in column {}, expected Float but got {:?}",
                        column_id, v
                    )
                    .into()),
                },
                InnerColumnType::BitFlag { values } => {
                    let v = match v {
                        Value::String(s) => match s.parse::<i64>() {
                            Ok(v) => v,
                            Err(e) => {
                                return Err(format!(
                                        "Validation error in column {}, expected BitFlag but got String that cannot be parsed: {}",
                                        column_id, e
                                    )
                                    .into());
                            }
                        },
                        Value::Number(v) => {
                            if v.is_i64() {
                                v.as_i64().unwrap()
                            } else {
                                return Err(format!(
                                    "Validation error in column {}, expected BitFlag but got Float",
                                    column_id
                                )
                                .into());
                            }
                        }
                        _ => {
                            return Err(format!(
                                "Validation error in column {}, expected BitFlag but got {:?}",
                                column_id, v
                            )
                            .into())
                        }
                    };

                    let mut final_value = 0;

                    // Set all the valid bits in final_value to ensure no unknown bits are being set
                    for (_, bit) in values.iter() {
                        if *bit & v == *bit {
                            final_value |= *bit;
                        }
                    }

                    if final_value == 0 {
                        // Set the first value as the default value
                        let Some(fv) = values.values().next() else {
                            return Err(
                                format!(
                                    "Validation error in column {}, expected BitFlag but no default value found",
                                    column_id
                                )
                                .into()
                            );
                        };

                        final_value = *fv;
                    }

                    Ok(Value::Number(final_value.into()))
                }
                InnerColumnType::Boolean {} => match v {
                    Value::String(s) => {
                        let value = match s.parse::<bool>() {
                            Ok(v) => v,
                            Err(e) => {
                                return Err(format!(
                                    "Validation error in column {}, expected Boolean but got String that cannot be parsed: {}",
                                    column_id, e
                                )
                                .into());
                            }
                        };

                        Ok(Value::Bool(value))
                    }
                    Value::Bool(v) => Ok(Value::Bool(v)),
                    _ => Err(format!(
                        "Validation error in column {}, expected Boolean but got {:?}",
                        column_id, v
                    )
                    .into()),
                },
                InnerColumnType::Json { max_bytes, .. } => {
                    // Convert back to json to get bytes
                    match v {
                        Value::String(s) => {
                            if s.len() > max_bytes.unwrap_or(0) {
                                return Err(
                                    format!(
                                        "Validation error in column {}, expected JSON with max bytes {} but got JSON with bytes {}",
                                        column_id, max_bytes.unwrap_or(0), s.len()
                                    )
                                    .into()
                                );
                            }

                            let v: serde_json::Value = {
                                if !s.starts_with("[") && !s.starts_with("{") {
                                    serde_json::Value::String(s)
                                } else {
                                    match serde_json::from_str(&s) {
                                        Ok(v) => v,
                                        Err(e) => {
                                            return Err(
                                                format!(
                                                    "Validation error in column {}, expected JSON but got String that cannot be parsed: {}",
                                                    column_id, e
                                                )
                                                .into()
                                            );
                                        }
                                    }
                                }
                            };

                            Ok(v)
                        }
                        _ => {
                            let bytes = match serde_json::to_string(&v) {
                                Ok(b) => b,
                                Err(e) => {
                                    return Err(
                                        format!(
                                            "Validation error in column {}, expected JSON but got value that cannot be converted to JSON: {}",
                                            column_id, e
                                        )
                                        .into()
                                    );
                                }
                            };

                            if let Some(max_bytes) = max_bytes {
                                if bytes.len() > *max_bytes {
                                    return Err(
                                        format!(
                                            "Validation error in column {}, expected JSON with max bytes {} but got JSON with bytes {}",
                                            column_id, max_bytes, bytes.len()
                                        )
                                        .into()
                                    );
                                }
                            }

                            Ok(v)
                        }
                    }
                }
            }
        }
        ColumnType::Array { inner } => match v {
            Value::Array(l) => {
                let mut values: Vec<Value> = Vec::new();

                let column_type = ColumnType::new_scalar(inner.clone());
                for v in l {
                    let new_v = validate_value(v, &column_type, column_id, nullable)?;

                    values.push(new_v);
                }

                Ok(Value::Array(values))
            }
            _ => Err(format!(
                "Validation error in column {}, expected Array but got {:?}",
                column_id, v
            )
            .into()),
        },
    }
}

/// Settings API: View implementation
pub async fn settings_view<T: Clone>(
    setting: &Setting<T>,
    data: &T,
    filters: indexmap::IndexMap<String, Value>, // The filters to apply
) -> Result<Vec<indexmap::IndexMap<String, Value>>, Error> {
    let Some(ref viewer) = setting.operations.view else {
        return Err(format!("Operation not supported: {}", OperationType::View).into());
    };

    let states = viewer.view(data, filters).await?;

    let mut values: Vec<indexmap::IndexMap<String, Value>> = Vec::new();

    for mut state in states {
        // We know that the columns are in the same order as the row
        for col in setting.columns.iter() {
            let mut val = state.swap_remove(&col.id).unwrap_or(Value::Null);

            // Validate the value
            val = validate_value(val, &col.column_type, &col.id, col.nullable)?;

            // Reinsert
            state.insert(col.id.to_string(), val);
        }

        // Remove ignored columns + secret columns now that the actions have been executed
        for col in setting.columns.iter() {
            if col.secret {
                state.swap_remove(&col.id);
                continue; // Skip secret columns in view. **this applies to view and update only as create is creating a new object**
            }

            if col.ignored_for.contains(&OperationType::View) {
                state.swap_remove(&col.id);
            }
        }

        values.push(state);
    }

    Ok(values)
}

/// Settings API: Create implementation
pub async fn settings_create<T: Clone>(
    setting: &Setting<T>,
    data: &T,
    fields: indexmap::IndexMap<String, Value>,
) -> Result<indexmap::IndexMap<String, Value>, Error> {
    let Some(ref creator) = setting.operations.create else {
        return Err(format!("Operation not supported: {}", OperationType::Create).into());
    };

    // Ensure all columns exist in fields, note that we can ignore extra fields so this one single loop is enough
    let mut state = fields;
    for column in setting.columns.iter() {
        if column.ignored_for.contains(&OperationType::Create) {
            continue;
        }

        // If the column is ignored for, only parse, otherwise parse and validate
        let value = {
            // Get the value
            let val = state.swap_remove(&column.id).unwrap_or(Value::Null);

            validate_value(val, &column.column_type, &column.id, column.nullable)?
        };

        state.insert(column.id.to_string(), value);
    }

    // Now execute all actions and handle null checks
    for column in setting.columns.iter() {
        // Checks should only happen if the column is not being intentionally ignored
        if column.ignored_for.contains(&OperationType::Create) {
            continue;
        }

        let Some(value) = state.get(&column.id) else {
            return Err(format!(
                "Internal error: Column `{}` not found in state despite just being parsed",
                column.id
            )
            .into());
        };

        // Check if the column is nullable
        if !column.nullable && matches!(value, Value::Null) {
            return Err(format!("Missing or invalid field: {}", column.id).into());
        }
    }

    // Remove ignored columns now that the actions have been executed
    for col in setting.columns.iter() {
        if col.ignored_for.contains(&OperationType::Create) {
            state.swap_remove(&col.id);
        }
    }

    let new_state = creator.create(data, state).await?;

    Ok(new_state)
}

/// Settings API: Update implementation
pub async fn settings_update<T: Clone>(
    setting: &Setting<T>,
    data: &T,
    fields: indexmap::IndexMap<String, Value>,
) -> Result<indexmap::IndexMap<String, Value>, Error> {
    let Some(ref updater) = setting.operations.update else {
        return Err(format!("Operation not supported: {}", OperationType::Update).into());
    };

    // Ensure all columns exist in fields, note that we can ignore extra fields so this one single loop is enough
    let mut state = fields;
    for column in setting.columns.iter() {
        if column.ignored_for.contains(&OperationType::Update) {
            continue;
        }

        // If the column is ignored for, only parse, otherwise parse and validate
        let value = {
            // Get the value
            let val = state.swap_remove(&column.id).unwrap_or(Value::Null);
            validate_value(val, &column.column_type, &column.id, column.nullable)?
        };

        state.insert(column.id.to_string(), value);
    }

    // Now execute all actions and handle null checks
    for column in setting.columns.iter() {
        // Checks should only happen if the column is not being intentionally ignored
        if column.ignored_for.contains(&OperationType::Update) {
            continue;
        }

        let Some(value) = state.get(&column.id) else {
            return Err(format!(
                "Internal error: Column `{}` not found in state despite just being parsed",
                column.id
            )
            .into());
        };

        // Check if the column is nullable
        if !column.nullable && matches!(value, Value::Null) {
            return Err(format!("Missing or invalid field: {}", column.id).into());
        }
    }

    // Remove ignored columns now that the actions have been executed
    for col in setting.columns.iter() {
        if col.ignored_for.contains(&OperationType::Update) {
            state.swap_remove(&col.id);
        }
    }

    let new_state = updater.update(data, state).await?;

    Ok(new_state)
}

/// Settings API: Delete implementation
#[allow(clippy::too_many_arguments)]
pub async fn settings_delete<T: Clone>(
    setting: &Setting<T>,
    data: &T,
    fields: indexmap::IndexMap<String, Value>,
) -> Result<(), Error> {
    let Some(ref deleter) = setting.operations.delete else {
        return Err(format!("Operation not supported: {}", OperationType::Delete).into());
    };

    let mut fields = fields;
    let mut state = indexmap::IndexMap::with_capacity(setting.columns.len());
    for column in setting.columns.iter() {
        if column.ignored_for.contains(&OperationType::Delete) || !column.primary_key {
            continue;
        }

        let Some(value) = fields.swap_remove(&column.id) else {
            return Err(format!("Missing or invalid required/primary key field: {}", column.id).into());
        };

        let value = validate_value(value, &column.column_type, &column.id, column.nullable)?;
        state.insert(column.id.to_string(), value);
    }

    deleter.delete(data, state).await?;

    Ok(())
}
