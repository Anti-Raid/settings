use std::sync::Arc;

use crate::types::{Column, ColumnType, InnerColumnType, OperationType, Setting};
use serde_json::{Number, Value};
use serenity::all::CommandOptionType;

/// Parse a numeric list from a string without knowing its separator
fn parse_numeric_list<T: std::str::FromStr + Send + Sync>(
    s: &str,
    replace: &[(&'static str, &'static str)],
) -> Result<Vec<T>, T::Err> {
    let mut list = Vec::new();
    let mut number = String::new();

    for c in s.chars() {
        if c.is_numeric() {
            number.push(c);
        } else if !number.is_empty() {
            for (from, to) in replace {
                number = number.replace(from, to);
            }
            list.push(number.parse::<T>()?);
            number.clear();
        }
    }

    if !number.is_empty() {
        list.push(number.parse::<T>()?);
    }

    Ok(list)
}

fn split_input_to_string(s: &str, separator: &str) -> Vec<String> {
    s.split(separator)
        .filter_map(|s| {
            let s = s.trim();
            if s.is_empty() {
                None
            } else {
                Some(s.to_string())
            }
        })
        .collect()
}

/// Given a set of bitflag values and an input, return the bitflag value
#[inline]
fn convert_bitflags_string_to_value(
    values: &indexmap::IndexMap<String, i64>,
    input: Option<String>,
) -> Value {
    match input {
        Some(input) => {
            let mut bitflags = 0;

            for value in input.split(';') {
                if let Some(value) = values.get(value) {
                    bitflags |= *value;
                }
            }

            Value::Number(bitflags.into())
        }
        None => Value::Null,
    }
}

/// This function takes in a serenity ResolvedValue and a ColumnType and returns a Value
fn serenity_resolvedvalue_to_value(
    rv: &serenity::all::ResolvedValue<'_>,
    column_type: &ColumnType,
) -> Result<Value, crate::Error> {
    // Before checking column_type, first handle unresolved resolved values so they don't waste our time
    #[allow(clippy::single_match)]
    match rv {
        serenity::all::ResolvedValue::Unresolved(inner) => match inner {
            serenity::all::Unresolved::Attachment(aid) => {
                return Ok(Value::String(aid.to_string()));
            }
            serenity::all::Unresolved::Channel(id) => {
                return Ok(Value::String(id.to_string()));
            }
            serenity::all::Unresolved::Mentionable(id) => {
                return Ok(Value::String(id.to_string()));
            }
            serenity::all::Unresolved::RoleId(id) => {
                return Ok(Value::String(id.to_string()));
            }
            serenity::all::Unresolved::User(id) => {
                return Ok(Value::String(id.to_string()));
            }
            serenity::all::Unresolved::Unknown(_) => {
                return Ok(Value::Null);
            }
            _ => {}
        },
        _ => {}
    }

    // Now handle the actual conversion code
    //
    // Get the inner column type and is_array status
    let (is_array, inner_column_type) = match column_type {
        ColumnType::Scalar { ref inner } => (false, inner),
        ColumnType::Array { ref inner } => (true, inner),
    };

    let pot_output = {
        match rv {
            serenity::all::ResolvedValue::Boolean(v) => v.to_string(),
            serenity::all::ResolvedValue::Integer(v) => v.to_string(),
            serenity::all::ResolvedValue::Number(v) => v.to_string(),
            serenity::all::ResolvedValue::String(v) => v.to_string(),
            serenity::all::ResolvedValue::Attachment(v) => v.proxy_url.to_string(),
            serenity::all::ResolvedValue::Channel(v) => v.id.to_string(),
            serenity::all::ResolvedValue::Role(v) => v.id.to_string(),
            serenity::all::ResolvedValue::User(v, _) => v.id.to_string(),
            _ => {
                return Err(format!(
                    "Please report: INTERNAL: Got unsupported ResolvedValue: {:?}",
                    rv
                )
                .into())
            }
        }
    };

    match inner_column_type {
        InnerColumnType::Integer {} => {
            if is_array {
                // Handle integer list
                let list = parse_numeric_list::<i64>(&pot_output, &[])?;

                let mut new_list = Vec::new();

                for v in list {
                    new_list.push(Value::Number(v.into()));
                }

                return Ok(Value::Array(new_list));
            } else {
                match rv {
                    serenity::all::ResolvedValue::Integer(v) => {
                        return Ok(Value::Number((*v).into()));
                    }
                    _ => return Err("Expected integer, got something else".into()),
                }
            }
        }
        InnerColumnType::Float {} => {
            if is_array {
                // Handle integer list
                let list = parse_numeric_list::<f64>(&pot_output, &[])?;

                let mut new_list = Vec::new();

                for v in list {
                    new_list.push(Value::Number(
                        Number::from_f64(v).ok_or("Failed to convert to f64")?,
                    ));
                }

                return Ok(Value::Array(new_list));
            } else {
                match rv {
                    serenity::all::ResolvedValue::Number(v) => {
                        return Ok(Value::Number(
                            Number::from_f64(*v).ok_or("Failed to convert to f64")?,
                        ));
                    }
                    _ => return Err("Expected float, got something else".into()),
                }
            }
        }
        InnerColumnType::Boolean {} => {
            if is_array {
                // Handle integer list
                let list = parse_numeric_list::<bool>(&pot_output, &[])?;

                let mut new_list = Vec::new();

                for v in list {
                    new_list.push(Value::Bool(v));
                }

                return Ok(Value::Array(new_list));
            } else {
                match rv {
                    serenity::all::ResolvedValue::Boolean(v) => {
                        return Ok(Value::Bool(*v));
                    }
                    _ => return Err("Expected boolean, got something else".into()),
                }
            }
        }
        InnerColumnType::String { .. } => {
            if !is_array {
                match rv {
                    serenity::all::ResolvedValue::String(v) => {
                        return Ok(Value::String(v.to_string()));
                    }
                    serenity::all::ResolvedValue::Role(v) => {
                        return Ok(Value::String(v.id.to_string()));
                    }
                    serenity::all::ResolvedValue::Channel(v) => {
                        return Ok(Value::String(v.id.to_string()));
                    }
                    _ => return Err("Expected string, got something else".into()),
                }
            }
        }
        InnerColumnType::BitFlag { ref values } => {
            if is_array {
                return Err("Array bitflags are not supported yet".into()); // TODO
            }

            match rv {
                serenity::all::ResolvedValue::String(v) => {
                    return Ok(convert_bitflags_string_to_value(
                        values,
                        Some(v.to_string()),
                    ));
                }
                _ => return Err("Expected string, got something else".into()),
            }
        }
        // Fallback to the fallback code
        _ => {}
    };

    // Fallback code
    if is_array {
        let list = split_input_to_string(&pot_output, ",");

        let mut new_list = Vec::new();

        for v in list {
            if v.is_empty() {
                continue;
            }

            new_list.push(Value::String(v));
        }

        Ok(Value::Array(new_list))
    } else {
        Ok(Value::String(pot_output))
    }
}

/// In order to provide state to the subcommand callback, we need to wrap it in a struct and then pass it through custom_data
pub struct SubcommandCallbackWrapper<Data: Clone> {
    pub config_option: Setting<Data>,
    pub data: Arc<Data>,
    pub operation_type: OperationType,
}

/// Gets the values from a serenity ResolvedValue handling choices and all that garbage
fn getvalues<Data: Clone>(
    config_opt: &Setting<Data>,
    interaction: &serenity::all::Interaction,
) -> Result<indexmap::IndexMap<String, Value>, crate::Error> {
    let resolved_args = match interaction {
        serenity::all::Interaction::Command(interaction) => interaction.data.options(),
        serenity::all::Interaction::Autocomplete(interaction) => interaction.data.options(),
        _ => return Err("Invalid interaction type".into()),
    };

    let Some(resolved_args) = resolved_args
        .into_iter()
        .find_map(|option| match option.value {
            serenity::all::ResolvedValue::SubCommand(o) => Some(o),
            serenity::all::ResolvedValue::SubCommandGroup(o) => {
                // Extract out the first subcommand
                if let Some(first) = o.into_iter().next() {
                    match first.value {
                        serenity::all::ResolvedValue::SubCommand(o)
                        | serenity::all::ResolvedValue::SubCommandGroup(o) => Some(o),
                        _ => None,
                    }
                } else {
                    None
                }
            }
            _ => None,
        })
    else {
        return Err("Invalid interaction data [expected subcommand or subcommand group]".into());
    };

    let mut map = indexmap::IndexMap::new();

    for column in config_opt.columns.iter() {
        let Some(arg) = resolved_args.iter().find(|a| a.name == column.id) else {
            continue; // Skip if the column is not present
        };

        let value = serenity_resolvedvalue_to_value(&arg.value, &column.column_type)
            .map_err(|e| format!("Column `{}`: {}", column.id, e))?;

        map.insert(column.id.to_string(), value);
    }

    Ok(map)
}

/// Subcommand callback
pub async fn subcommand_command<Data: Clone>(
    ctx: &serenity::all::Context,
    interaction: &serenity::all::Interaction,
    subcommand_callback_wrapper: &SubcommandCallbackWrapper<Data>,
) -> Result<(), crate::Error> {
    let cmd_interaction = match interaction {
        serenity::all::Interaction::Command(interaction) => interaction,
        _ => return Err("Invalid interaction type".into()),
    };
    match subcommand_callback_wrapper.operation_type {
        OperationType::View => {
            super::ui::settings_viewer(
                super::ui::Src::Interaction((cmd_interaction, ctx, cmd_interaction.user.id)),
                &subcommand_callback_wrapper.config_option,
                &subcommand_callback_wrapper.data,
                indexmap::IndexMap::new(), // TODO: Add filtering in the future
            )
            .await
        }
        OperationType::Create => {
            let entry = getvalues(&subcommand_callback_wrapper.config_option, interaction)?;

            super::ui::settings_creator(
                super::ui::Src::Interaction((cmd_interaction, ctx, cmd_interaction.user.id)),
                &subcommand_callback_wrapper.config_option,
                &subcommand_callback_wrapper.data,
                entry,
            )
            .await
        }
        OperationType::Update => {
            let mut entry = getvalues(&subcommand_callback_wrapper.config_option, interaction)?;

            // Attempt to autofill from created data if possible
            let mut have_found_for_autofill = false;
            if subcommand_callback_wrapper
                .config_option
                .operations
                .view
                .is_some()
            {
                let mut pkey_state = indexmap::IndexMap::new();
                for column in subcommand_callback_wrapper.config_option.columns.iter() {
                    if column.primary_key {
                        if let Some(value) = entry.get(&column.id) {
                            pkey_state.insert(column.id.clone(), value.clone());
                        } else {
                            return Err(format!(
                                "An input for `{}` is required",
                                column.id
                            )
                            .into());
                        }
                    }
                }

                let values = crate::cfg::settings_view(
                    &subcommand_callback_wrapper.config_option,
                    &subcommand_callback_wrapper.data,
                    indexmap::indexmap! {},
                )
                .await
                .map_err(|e| format!("Error fetching settings for autofill: {:?}", e))?;

                // Find value with primary key that matches the update
                for value in values {
                    for (pkey_key, pkey_value) in pkey_state.iter() {
                        if let Some(v) = value.get(pkey_key) {
                            if v != pkey_value {
                                continue;
                            }
                        } else {
                            continue;
                        }
                    }

                    have_found_for_autofill = true;

                    for (key, value) in value {
                        if entry.contains_key(&key) {
                            continue;
                        }

                        entry.insert(key, value);
                    }
                    break;
                }
            }

            if !have_found_for_autofill {
                // Switch to create impl
                return super::ui::settings_creator(
                    super::ui::Src::Interaction((cmd_interaction, ctx, cmd_interaction.user.id)),
                    &subcommand_callback_wrapper.config_option,
                    &subcommand_callback_wrapper.data,
                    entry,
                )
                .await;
            }

            super::ui::settings_updater(
                super::ui::Src::Interaction((cmd_interaction, ctx, cmd_interaction.user.id)),
                &subcommand_callback_wrapper.config_option,
                &subcommand_callback_wrapper.data,
                entry,
            )
            .await
        }
        OperationType::Delete => {
            let entry = getvalues(&subcommand_callback_wrapper.config_option, interaction)?;

            super::ui::settings_deleter(
                super::ui::Src::Interaction((cmd_interaction, ctx, cmd_interaction.user.id)),
                &subcommand_callback_wrapper.config_option,
                &subcommand_callback_wrapper.data,
                entry,
            )
            .await
        }
    }
}

/// An autocomplete callback
pub async fn subcommand_autocomplete<Data: Clone>(
    ctx: &serenity::all::Context,
    interaction: &serenity::all::Interaction,
    subcommand_callback_wrapper: SubcommandCallbackWrapper<Data>,
) -> Result<(), crate::Error> {
    let cmd_interaction = match interaction {
        serenity::all::Interaction::Autocomplete(interaction) => interaction,
        _ => return Err("Invalid interaction type".into()),
    };

    let Some(autocomplete_option) = cmd_interaction.data.autocomplete() else {
        return Err("Invalid interaction data [expected autocomplete]".into());
    };

    let columns = &subcommand_callback_wrapper.config_option.columns;
    let Some(column) = columns.iter().find(|c| c.id == autocomplete_option.name) else {
        return Err("Invalid column".into());
    };

    let options = match &column.column_type {
        ColumnType::Scalar { inner } => match inner {
            InnerColumnType::String { allowed_values, .. } => {
                let mut choices = Vec::new();

                for value in allowed_values {
                    if value.contains(autocomplete_option.value) {
                        choices.push(serenity::all::AutocompleteChoice::new(
                            value.clone(),
                            value.clone(),
                        ));
                    }
                }

                choices
            }
            _ => return Ok(()),
        },
        ColumnType::Array { inner } => match inner {
            InnerColumnType::String { allowed_values, .. } => {
                let mut choices = Vec::new();

                let autocomp_values = split_input_to_string(autocomplete_option.value, ",");
                let last_value = match autocomp_values.last() {
                    Some(v) => v,
                    None => &"".to_string(),
                };

                for value in allowed_values {
                    if value.contains(last_value) {
                        if autocomp_values.len() <= 1 {
                            choices.push(serenity::all::AutocompleteChoice::new(
                                value.clone(),
                                value.clone(),
                            ));
                            continue;
                        } else {
                            let autocomplete_choice_value = format!(
                                "{},{}",
                                autocomp_values[..autocomp_values.len() - 1].join(","),
                                value
                            );
                            choices.push(serenity::all::AutocompleteChoice::new(
                                autocomplete_choice_value.clone(),
                                autocomplete_choice_value,
                            ));
                        }
                    }
                }

                choices
            }
            _ => return Ok(()),
        },
    };

    cmd_interaction
        .create_response(
            &ctx.http,
            serenity::all::CreateInteractionResponse::Autocomplete(
                serenity::all::CreateAutocompleteResponse::new().set_choices({
                    if options.len() > 25 {
                        options[..25].to_vec()
                    } else {
                        options
                    }
                }),
            ),
        )
        .await?;

    Ok(())
}

/// Create a command from a setting
pub fn create_commands_from_setting<'a, Data: Clone>(
    setting: &Setting<Data>,
) -> serenity::all::CreateCommand<'a> {
    let cmd = serenity::all::CreateCommand::new(setting.id.to_string())
        .description({
            if setting.description.len() > 100 {
                setting.description[..97].to_string() + "..."
            } else {
                setting.description.to_string()
            }
        })
        .kind(serenity::all::CommandType::ChatInput)
        .integration_types(vec![serenity::all::InstallationContext::Guild])
        .set_options(create_subcommands_from_setting(setting));

    cmd
}

/// Create a command from a setting with a root command. This will use a subcommand group
/// which contains the subcommands for adding, updating, deleting, and viewing the setting
pub fn create_commands_from_setting_with_root<'a, Data: Clone>(
    setting: &Setting<Data>,
    root: serenity::all::CreateCommand<'a>,
) -> serenity::all::CreateCommand<'a> {
    let subcommands = create_subcommands_from_setting(setting);

    let subcommand_group = serenity::all::CreateCommandOption::new(
        CommandOptionType::SubCommandGroup,
        setting.id.to_string(),
        {
            if setting.description.len() > 50 {
                setting.description[..47].to_string() + "..."
            } else {
                setting.description.to_string()
            }
        },
    )
    .set_sub_options(subcommands);

    root.add_option(subcommand_group)
}

fn create_subcommands_from_setting<'a, Data: Clone>(
    config_opt: &Setting<Data>,
) -> Vec<serenity::all::CreateCommandOption<'a>> {
    let mut sub_cmds = Vec::new();

    // Create subcommands
    if config_opt.operations.view.is_some() {
        sub_cmds.push(create_command_for_operation_type(
            config_opt,
            OperationType::View,
        ));
    }
    if config_opt.operations.create.is_some() {
        sub_cmds.push(create_command_for_operation_type(
            config_opt,
            OperationType::Create,
        ));
    }
    if config_opt.operations.update.is_some() {
        sub_cmds.push(create_command_for_operation_type(
            config_opt,
            OperationType::Update,
        ));
    }
    if config_opt.operations.delete.is_some() {
        sub_cmds.push(create_command_for_operation_type(
            config_opt,
            OperationType::Delete,
        ));
    }

    sub_cmds
}

/// Get the choices from the column_type. Note that only string scalar columns can have choices
fn get_string_choices_for_column(column: &Column) -> Option<Vec<String>> {
    // Get the choices from the column_type. Note that only string scalar columns can have choices
    #[allow(clippy::collapsible_match)]
    match column.column_type {
        ColumnType::Scalar { ref inner } => {
            match inner {
                InnerColumnType::String { allowed_values, .. } => {
                    if allowed_values.is_empty() || allowed_values.len() > 25 {
                        None
                    } else {
                        Some(allowed_values.clone())
                    }
                }
                _ => None, // No other channel type can contain a scalar
            }
        }
        _ => None,
    }
}

fn is_column_required_for_operation_type(
    column: &Column,
    operation_type: OperationType,
) -> bool {
    if operation_type == OperationType::Update && !column.primary_key {
        return false;
    }

    !column.nullable
}

fn create_command_for_operation_type<'a, Data: Clone>(
    config_opt: &Setting<Data>,
    operation_type: OperationType,
) -> serenity::all::CreateCommandOption<'a> {
    let mut args = serenity::all::CreateCommandOption::new(
        serenity::all::CommandOptionType::SubCommand,
        match operation_type {
            OperationType::View => "view",
            OperationType::Create => "create",
            OperationType::Update => "update",
            OperationType::Delete => "delete",
        },
        {
            if config_opt.description.len() > 50 {
                config_opt.description[..47].to_string() + "..."
            } else {
                config_opt.description.to_string()
            }
        },
    );

    if operation_type == OperationType::View {
        return args; // View doesnt need any arguments
    }

    // Sort the columns so required options come first
    let mut sort_idx = vec![];

    for (idx, column) in config_opt.columns.iter().enumerate() {
        if operation_type == OperationType::Delete && !column.primary_key {
            continue; // Skip if not the primary key
        }

        if !is_column_required_for_operation_type(column, operation_type) {
            sort_idx.push(idx);
        } else {
            sort_idx.insert(0, idx);
        }
    }

    for idx in sort_idx {
        let column = &config_opt.columns[idx];

        // Check if we should ignore this column
        if column.ignored_for.contains(&operation_type) {
            continue;
        }

        // Add the new command parameter
        let arg = serenity::all::CreateCommandOption::new(
            {
                match column.column_type {
                    ColumnType::Scalar { ref inner } => {
                        match inner {
                            InnerColumnType::Integer {} => {
                                serenity::all::CommandOptionType::Integer
                            }
                            InnerColumnType::Float {} => serenity::all::CommandOptionType::Number,
                            InnerColumnType::Boolean {} => {
                                serenity::all::CommandOptionType::Boolean
                            }
                            InnerColumnType::String { kind, .. } => match kind.as_str() {
                                "channel" => serenity::all::CommandOptionType::Channel,
                                "user" => serenity::all::CommandOptionType::User,
                                "role" => serenity::all::CommandOptionType::Role,
                                // Fallback to string
                                _ => serenity::all::CommandOptionType::String,
                            },
                            // Fallback to string
                            _ => serenity::all::CommandOptionType::String,
                        }
                    }
                    // Other types are handled automatically in validate so we should fallback to string
                    _ => serenity::all::CommandOptionType::String,
                }
            },
            column.id.to_string(),
            {
                if column.description.len() > 100 {
                    column.description[..97].to_string() + "..."
                } else {
                    column.description.to_string()
                }
            },
        )
        .required(is_column_required_for_operation_type(
            column,
            operation_type,
        ))
        .set_autocomplete(field_supports_autocomplete(column));

        // add string choice
        let arg = match get_string_choices_for_column(column) {
            Some(choices) => {
                let mut arg = arg;
                for choice in choices {
                    arg = arg.add_string_choice(choice.clone(), choice);
                }
                arg
            }
            None => arg,
        };

        args = args.add_sub_option(arg);
    }

    args
}

fn field_supports_autocomplete(field: &Column) -> bool {
    match &field.column_type {
        ColumnType::Scalar { ref inner } => match inner {
            InnerColumnType::String { allowed_values, .. } => allowed_values.len() > 25,
            _ => false,
        },
        ColumnType::Array { inner } => {
            match inner {
                InnerColumnType::String { allowed_values, .. } => !allowed_values.is_empty(), // Arrays do benefit from autocomplete
                _ => false,
            }
        }
    }
}
