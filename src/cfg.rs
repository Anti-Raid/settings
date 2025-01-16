use super::types::SettingsError;
use super::types::{ColumnType, InnerColumnType, OperationType, Setting};
use crate::value::Value;

#[derive(PartialEq, Debug)]
enum Unit {
    Seconds,
    Minutes,
    Hours,
    Days,
    Weeks,
}

impl Unit {
    /// Convert the unit to seconds
    pub fn to_seconds(&self) -> u64 {
        match self {
            Unit::Seconds => 1,
            Unit::Minutes => 60,
            Unit::Hours => 3600,
            Unit::Days => 86400,
            Unit::Weeks => 604800,
        }
    }
}

impl TryFrom<&str> for Unit {
    type Error = crate::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "seconds" => Ok(Unit::Seconds),
            "second" => Ok(Unit::Seconds), // Allow "second" as a shorthand for "seconds"
            "secs" => Ok(Unit::Seconds),   // Allow "secs" as a shorthand for "seconds"
            "sec" => Ok(Unit::Seconds),    // Allow "sec" as a shorthand for "seconds"
            "s" => Ok(Unit::Seconds),      // Allow "s" as a shorthand for "seconds"
            "minutes" => Ok(Unit::Minutes),
            "minute" => Ok(Unit::Minutes), // Allow "minute" as a shorthand for "minutes"
            "mins" => Ok(Unit::Minutes),   // Allow "mins" as a shorthand for "minutes"
            "min" => Ok(Unit::Minutes),    // Allow "min" as a shorthand for "minutes"
            "m" => Ok(Unit::Minutes),      // Allow "m" as a shorthand for "minutes"
            "hours" => Ok(Unit::Hours),
            "hour" => Ok(Unit::Hours), // Allow "hour" as a shorthand for "hours"
            "hrs" => Ok(Unit::Hours),  // Allow "hrs" as a shorthand for "hours"
            "hr" => Ok(Unit::Hours),   // Allow "hr" as a shorthand for "hours"
            "h" => Ok(Unit::Hours),    // Allow "h" as a shorthand for "hours"
            "days" => Ok(Unit::Days),
            "day" => Ok(Unit::Days), // Allow "day" as a shorthand for "days"
            "d" => Ok(Unit::Days),   // Allow "d" as a shorthand for "days"
            "weeks" => Ok(Unit::Weeks),
            "week" => Ok(Unit::Weeks), // Allow "week" as a shorthand for "weeks"
            "w" => Ok(Unit::Weeks),    // Allow "w" as a shorthand for "weeks"
            _ => Err("Invalid unit".into()),
        }
    }
}

/// Given a string of the format <number> days/hours/minutes/seconds, parse it into a u64 of seconds
///
/// This function should handle both spaced and non-spaced formats
fn parse_duration_string(s: &str) -> Result<(u64, Unit), crate::Error> {
    let mut number: u64 = 0;
    let mut unit = String::new();

    // Keep looping adding up each number until we hit a non-number which gets added to unit
    for c in s.chars() {
        if c.is_numeric() {
            number = number * 10 + c.to_digit(10).ok_or("Cannot convert to integer")? as u64;
        } else {
            if c == ' ' {
                continue;
            }

            unit.push(c);
        }
    }

    let unit = Unit::try_from(unit.as_str())?;

    Ok((number, unit))
}

/// Given a string of the format <number> days/hours/minutes/seconds, parse it into a chrono::Duration
///
/// This is a wrapper around parse_duration_string that converts the result into a chrono::Duration
fn parse_duration_string_to_chrono_duration(s: &str) -> Result<chrono::Duration, crate::Error> {
    let (number, unit) = parse_duration_string(s)?;

    Ok(chrono::Duration::from_std(std::time::Duration::from_secs(
        number * unit.to_seconds(),
    ))?)
}

/// Parse a value against the schema's column type
fn _parse_value(
    v: Value,
    column_type: &ColumnType,
    column_id: &str,
) -> Result<Value, SettingsError> {
    match column_type {
        ColumnType::Scalar { inner } => {
            // Special case: JSON columns can be any type
            if matches!(v, Value::List(_)) && !matches!(inner, InnerColumnType::Json { .. }) {
                return Err(SettingsError::SchemaTypeValidationError {
                    column: column_id.to_string(),
                    expected_type: "Scalar".to_string(),
                    got_type: "Array".to_string(),
                });
            }

            match inner {
                InnerColumnType::Uuid {} => match v {
                    Value::String(s) => {
                        let value = s.parse::<uuid::Uuid>().map_err(|e| {
                            SettingsError::SchemaCheckValidationError {
                                column: column_id.to_string(),
                                check: "uuid_parse".to_string(),
                                accepted_range: "Valid UUID".to_string(),
                                error: e.to_string(),
                            }
                        })?;

                        Ok(Value::Uuid(value))
                    }
                    Value::Uuid(_) => Ok(v),
                    Value::None => Ok(v),
                    _ => Err(SettingsError::SchemaTypeValidationError {
                        column: column_id.to_string(),
                        expected_type: "Uuid".to_string(),
                        got_type: format!("{:?}", v),
                    }),
                },
                InnerColumnType::String { .. } => match v {
                    Value::String(_) => Ok(v),
                    Value::Uuid(v) => Ok(Value::String(v.to_string())),
                    Value::None => Ok(v),
                    _ => Err(SettingsError::SchemaTypeValidationError {
                        column: column_id.to_string(),
                        expected_type: "String".to_string(),
                        got_type: format!("{:?}", v),
                    }),
                },
                InnerColumnType::Timestamp {} => match v {
                    Value::String(s) => {
                        let value = chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S")
                            .map_err(|e| SettingsError::SchemaCheckValidationError {
                                column: column_id.to_string(),
                                check: "timestamp_parse".to_string(),
                                accepted_range: "Valid timestamp".to_string(),
                                error: e.to_string(),
                            })?;

                        Ok(Value::Timestamp(value))
                    }
                    Value::Timestamp(_) => Ok(v),
                    Value::None => Ok(v),
                    Value::TimestampTz(v) => Ok(Value::Timestamp(v.naive_utc())),
                    _ => Err(SettingsError::SchemaTypeValidationError {
                        column: column_id.to_string(),
                        expected_type: "Timestamp".to_string(),
                        got_type: format!("{:?}", v),
                    }),
                },
                InnerColumnType::TimestampTz {} => match v {
                    Value::String(s) => {
                        let value = chrono::DateTime::parse_from_rfc3339(&s).map_err(|e| {
                            SettingsError::SchemaCheckValidationError {
                                column: column_id.to_string(),
                                check: "timestamp_tz_parse".to_string(),
                                accepted_range: "Valid timestamp with timezone".to_string(),
                                error: e.to_string(),
                            }
                        })?;

                        // Convert value to DateTime<Utc> from DateTime<FixedOffset>
                        let value: chrono::DateTime<chrono::Utc> =
                            chrono::DateTime::from_naive_utc_and_offset(
                                value.naive_utc(),
                                chrono::Utc,
                            );

                        Ok(Value::TimestampTz(value))
                    }
                    Value::Timestamp(v) => Ok(Value::TimestampTz(
                        chrono::DateTime::from_naive_utc_and_offset(v, chrono::Utc),
                    )),
                    Value::TimestampTz(_) => Ok(v),
                    Value::None => Ok(v),
                    _ => Err(SettingsError::SchemaTypeValidationError {
                        column: column_id.to_string(),
                        expected_type: "TimestampTz".to_string(),
                        got_type: format!("{:?}", v),
                    }),
                },
                InnerColumnType::Interval {} => match v {
                    Value::String(s) => {
                        let dur = parse_duration_string_to_chrono_duration(&s).map_err(|e| {
                            SettingsError::SchemaCheckValidationError {
                                column: column_id.to_string(),
                                check: "interval_parse".to_string(),
                                accepted_range: "Valid interval".to_string(),
                                error: e.to_string(),
                            }
                        })?;

                        Ok(Value::Interval(dur))
                    }
                    Value::Integer(v) => {
                        let duration = chrono::Duration::seconds(v);
                        Ok(Value::Interval(duration))
                    }
                    Value::Interval(_) => Ok(v),
                    Value::None => Ok(v),
                    _ => Err(SettingsError::SchemaTypeValidationError {
                        column: column_id.to_string(),
                        expected_type: "Interval".to_string(),
                        got_type: format!("{:?}", v),
                    }),
                },
                InnerColumnType::Integer {} => match v {
                    Value::String(s) => {
                        if s.is_empty() {
                            Ok(Value::None)
                        } else {
                            let value = s.parse::<i64>().map_err(|e| {
                                SettingsError::SchemaCheckValidationError {
                                    column: column_id.to_string(),
                                    check: "integer_parse".to_string(),
                                    accepted_range: "Valid integer".to_string(),
                                    error: e.to_string(),
                                }
                            })?;

                            Ok(Value::Integer(value))
                        }
                    }
                    Value::Integer(v) => Ok(Value::Integer(v)),
                    Value::None => Ok(v),
                    _ => Err(SettingsError::SchemaTypeValidationError {
                        column: column_id.to_string(),
                        expected_type: "Integer".to_string(),
                        got_type: format!("{:?}", v),
                    }),
                },
                InnerColumnType::Float {} => match v {
                    Value::String(s) => {
                        let value = s.parse::<f64>().map_err(|e| {
                            SettingsError::SchemaCheckValidationError {
                                column: column_id.to_string(),
                                check: "float_parse".to_string(),
                                accepted_range: "Valid float".to_string(),
                                error: e.to_string(),
                            }
                        })?;

                        Ok(Value::Float(value))
                    }
                    Value::Float(v) => Ok(Value::Float(v)),
                    Value::None => Ok(v),
                    _ => Err(SettingsError::SchemaTypeValidationError {
                        column: column_id.to_string(),
                        expected_type: "Float".to_string(),
                        got_type: format!("{:?}", v),
                    }),
                },
                InnerColumnType::BitFlag { values } => match v {
                    Value::Integer(v) => {
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
                                return Err(SettingsError::SchemaCheckValidationError {
                                    column: column_id.to_string(),
                                    check: "bitflag_default".to_string(),
                                    accepted_range: "Valid bitflag".to_string(),
                                    error: "No default value found".to_string(),
                                });
                            };

                            final_value = *fv;
                        }

                        Ok(Value::Integer(final_value))
                    }
                    Value::String(s) => {
                        if s.is_empty() {
                            // Set the first value as the default value
                            let Some(fv) = values.values().next() else {
                                return Err(SettingsError::SchemaCheckValidationError {
                                    column: column_id.to_string(),
                                    check: "bitflag_default".to_string(),
                                    accepted_range: "Valid bitflag".to_string(),
                                    error: "No default value found".to_string(),
                                });
                            };

                            Ok(Value::Integer(*fv))
                        } else {
                            let v = s.parse::<i64>().map_err(|e| {
                                SettingsError::SchemaCheckValidationError {
                                    column: column_id.to_string(),
                                    check: "bitflag_parse".to_string(),
                                    accepted_range: "Valid bitflag".to_string(),
                                    error: e.to_string(),
                                }
                            })?;

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
                                    return Err(SettingsError::SchemaCheckValidationError {
                                        column: column_id.to_string(),
                                        check: "bitflag_default".to_string(),
                                        accepted_range: "Valid bitflag".to_string(),
                                        error: "No default value found".to_string(),
                                    });
                                };

                                final_value = *fv;
                            }

                            Ok(Value::Integer(final_value))
                        }
                    }
                    Value::None => Ok(v),
                    _ => Err(SettingsError::SchemaTypeValidationError {
                        column: column_id.to_string(),
                        expected_type: "Integer".to_string(),
                        got_type: format!("{:?}", v),
                    }),
                },
                InnerColumnType::Boolean {} => match v {
                    Value::String(s) => {
                        let value = s.parse::<bool>().map_err(|e| {
                            SettingsError::SchemaCheckValidationError {
                                column: column_id.to_string(),
                                check: "boolean_parse".to_string(),
                                accepted_range: "Valid boolean".to_string(),
                                error: e.to_string(),
                            }
                        })?;

                        Ok(Value::Boolean(value))
                    }
                    Value::Boolean(v) => Ok(Value::Boolean(v)),
                    Value::None => Ok(v),
                    _ => Err(SettingsError::SchemaTypeValidationError {
                        column: column_id.to_string(),
                        expected_type: "Boolean".to_string(),
                        got_type: format!("{:?}", v),
                    }),
                },
                InnerColumnType::Json { max_bytes } => {
                    // Convert back to json to get bytes
                    match v {
                        Value::String(s) => {
                            if s.len() > max_bytes.unwrap_or(0) {
                                return Err(SettingsError::SchemaCheckValidationError {
                                    column: column_id.to_string(),
                                    check: "json_max_bytes".to_string(),
                                    accepted_range: format!("<{}", max_bytes.unwrap_or(0)),
                                    error: format!("s.len() > *max_bytes: {}", s.len()),
                                });
                            }

                            let v: serde_json::Value = {
                                if !s.starts_with("[") && !s.starts_with("{") {
                                    serde_json::Value::String(s)
                                } else {
                                    serde_json::from_str(&s).map_err(|e| {
                                        SettingsError::SchemaCheckValidationError {
                                            column: column_id.to_string(),
                                            check: "json_parse".to_string(),
                                            accepted_range: "Valid JSON".to_string(),
                                            error: e.to_string(),
                                        }
                                    })?
                                }
                            };

                            Ok(Value::Json(v))
                        }
                        _ => {
                            let bytes = serde_json::to_string(&v.to_json()).map_err(|e| {
                                SettingsError::SchemaCheckValidationError {
                                    column: column_id.to_string(),
                                    check: "json_parse".to_string(),
                                    accepted_range: "Valid JSON".to_string(),
                                    error: e.to_string(),
                                }
                            })?;

                            if let Some(max_bytes) = max_bytes {
                                if bytes.len() > *max_bytes {
                                    return Err(SettingsError::SchemaCheckValidationError {
                                        column: column_id.to_string(),
                                        check: "json_max_bytes".to_string(),
                                        accepted_range: format!("<{}", max_bytes),
                                        error: format!("json.len() > *max_bytes: {}", bytes.len()),
                                    });
                                }
                            }

                            Ok(v)
                        }
                    }
                }
            }
        }
        ColumnType::Array { inner } => {
            if let InnerColumnType::Json { max_bytes } = inner {
                // Convert back to json to get bytes of the full payload as a whole
                let json = serde_json::to_string(&v.to_json()).map_err(|e| {
                    SettingsError::SchemaCheckValidationError {
                        column: column_id.to_string(),
                        check: "json_parse".to_string(),
                        accepted_range: "Valid JSON".to_string(),
                        error: e.to_string(),
                    }
                })?;

                if let Some(max_bytes) = max_bytes {
                    if json.len() > *max_bytes {
                        return Err(SettingsError::SchemaCheckValidationError {
                            column: column_id.to_string(),
                            check: "json_max_bytes".to_string(),
                            accepted_range: format!("<{}", max_bytes),
                            error: format!("json.len() > *max_bytes: {}", json.len()),
                        });
                    }
                }
            }

            match v {
                Value::List(l) => {
                    let mut values: Vec<Value> = Vec::new();

                    let column_type = ColumnType::new_scalar(inner.clone());
                    for v in l {
                        let new_v = _parse_value(v, &column_type, column_id)?;

                        values.push(new_v);
                    }

                    Ok(Value::List(values))
                }
                Value::None => Ok(v),
                _ => Err(SettingsError::SchemaTypeValidationError {
                    column: column_id.to_string(),
                    expected_type: "Array".to_string(),
                    got_type: format!("{:?}", v),
                }),
            }
        }
    }
}

/// Validates the value against the schema's column type
///
/// NOTE: This may make HTTP/Discord API requests to parse values such as channels etc.
#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
fn _validate_value(
    v: Value,
    column_type: &ColumnType,
    column_id: &str,
    is_nullable: bool,
) -> Result<Value, SettingsError> {
    let v = match column_type {
        ColumnType::Scalar { inner } => {
            // Special case: JSON columns can be any type
            if matches!(v, Value::List(_)) && !matches!(inner, InnerColumnType::Json { .. }) {
                return Err(SettingsError::SchemaTypeValidationError {
                    column: column_id.to_string(),
                    expected_type: "Scalar".to_string(),
                    got_type: "Array".to_string(),
                });
            }

            match inner {
                InnerColumnType::String {
                    min_length,
                    max_length,
                    allowed_values,
                    ..
                } => match v {
                    Value::String(ref s) => {
                        if let Some(min) = min_length {
                            if s.len() < *min {
                                return Err(SettingsError::SchemaCheckValidationError {
                                    column: column_id.to_string(),
                                    check: "minlength".to_string(),
                                    accepted_range: format!(">{}", min),
                                    error: "s.len() < *min".to_string(),
                                });
                            }
                        }

                        if let Some(max) = max_length {
                            if s.len() > *max {
                                return Err(SettingsError::SchemaCheckValidationError {
                                    column: column_id.to_string(),
                                    check: "maxlength".to_string(),
                                    accepted_range: format!("<{}", max),
                                    error: "s.len() > *max".to_string(),
                                });
                            }
                        }

                        if !allowed_values.is_empty() && !allowed_values.contains(s) {
                            return Err(SettingsError::SchemaCheckValidationError {
                                    column: column_id.to_string(),
                                    check: "allowed_values".to_string(),
                                    accepted_range: format!("{:?}", allowed_values),
                                    error: "!allowed_values.is_empty() && !allowed_values.contains(&s.as_str())".to_string()
                                });
                        }

                        Ok(v)
                    }
                    Value::None => {
                        if !is_nullable {
                            return Err(SettingsError::SchemaNullValueValidationError {
                                column: column_id.to_string(),
                            });
                        }

                        return Ok(v);
                    }
                    _ => Err(SettingsError::SchemaTypeValidationError {
                        column: column_id.to_string(),
                        expected_type: "String".to_string(),
                        got_type: format!("{:?}", v),
                    }),
                },
                _ => Ok(v),
            }
        }
        ColumnType::Array { inner } => match v {
            Value::List(l) => {
                let mut values: Vec<Value> = Vec::new();

                let column_type = ColumnType::new_scalar(inner.clone());
                for v in l {
                    let new_v = _validate_value(v, &column_type, column_id, is_nullable)?;

                    values.push(new_v);
                }

                Ok(Value::List(values))
            }
            Value::None => Ok(v),
            _ => Err(SettingsError::SchemaTypeValidationError {
                column: column_id.to_string(),
                expected_type: "Array".to_string(),
                got_type: format!("{:?}", v),
            }),
        },
    }?;

    if matches!(v, Value::None) && !is_nullable {
        return Err(SettingsError::SchemaNullValueValidationError {
            column: column_id.to_string(),
        });
    }

    Ok(v)
}

/// Settings API: View implementation
pub async fn settings_view<T: Clone>(
    setting: &Setting<T>,
    data: &T,
    filters: indexmap::IndexMap<String, Value>, // The filters to apply
) -> Result<Vec<indexmap::IndexMap<String, Value>>, SettingsError> {
    let Some(ref viewer) = setting.operations.view else {
        return Err(SettingsError::OperationNotSupported {
            operation: OperationType::View,
        });
    };

    let states = viewer.view(data, filters).await?;

    let mut values: Vec<indexmap::IndexMap<String, Value>> = Vec::new();

    for mut state in states {
        // We know that the columns are in the same order as the row
        for col in setting.columns.iter() {
            let mut val = state.swap_remove(&col.id).unwrap_or(Value::None);

            // Validate the value. returning the parsed value
            val = _parse_value(val, &col.column_type, &col.id)?;

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
) -> Result<indexmap::IndexMap<String, Value>, SettingsError> {
    let Some(ref creator) = setting.operations.create else {
        return Err(SettingsError::OperationNotSupported {
            operation: OperationType::Create,
        });
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
            let val = state.swap_remove(&column.id).unwrap_or(Value::None);

            // Validate and parse the value
            let parsed_value = _parse_value(val, &column.column_type, &column.id)?;
            _validate_value(
                parsed_value,
                &column.column_type,
                &column.id,
                column.nullable,
            )?
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
            return Err(SettingsError::Generic {
                message: format!(
                    "Column `{}` not found in state despite just being parsed",
                    column.id
                ),
                src: "settings_create [ext_checks]".to_string(),
                typ: "internal".to_string(),
            });
        };

        // Check if the column is nullable
        if !column.nullable && matches!(value, Value::None) {
            return Err(SettingsError::MissingOrInvalidField {
                field: column.id.to_string(),
                src: "settings_create [null check]".to_string(),
            });
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
) -> Result<indexmap::IndexMap<String, Value>, SettingsError> {
    let Some(ref updater) = setting.operations.update else {
        return Err(SettingsError::OperationNotSupported {
            operation: OperationType::Update,
        });
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
            let val = state.swap_remove(&column.id).unwrap_or(Value::None);

            // Validate and parse the value
            let parsed_value = _parse_value(val, &column.column_type, &column.id)?;
            _validate_value(
                parsed_value,
                &column.column_type,
                &column.id,
                column.nullable,
            )?
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
            return Err(SettingsError::Generic {
                message: format!(
                    "Column `{}` not found in state despite just being parsed",
                    column.id
                ),
                src: "settings_update [ext_checks]".to_string(),
                typ: "internal".to_string(),
            });
        };

        // Check if the column is nullable
        if !column.nullable && matches!(value, Value::None) {
            return Err(SettingsError::MissingOrInvalidField {
                field: column.id.to_string(),
                src: "settings_create [null check]".to_string(),
            });
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
    pkey: Value,
) -> Result<(), SettingsError> {
    let Some(ref deleter) = setting.operations.delete else {
        return Err(SettingsError::OperationNotSupported {
            operation: OperationType::Delete,
        });
    };

    let Some(pkey_column) = setting.columns.iter().find(|c| c.id == setting.primary_key) else {
        return Err(SettingsError::Generic {
            message: "Primary key column not found".to_string(),
            src: "settings_update [pkey_column_let_else]".to_string(),
            typ: "internal".to_string(),
        });
    };

    let pkey = _parse_value(pkey, &pkey_column.column_type, &setting.primary_key)?;

    deleter.delete(data, pkey).await?;

    Ok(())
}
