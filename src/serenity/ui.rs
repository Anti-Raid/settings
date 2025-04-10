use crate::cfg::{settings_create, settings_delete, settings_update, settings_view};
use crate::types::{ColumnType, InnerColumnType, Setting};
use serde_json::Value;
use serenity::all::CreateMessage;
use serenity::futures::StreamExt;
use std::time::Duration;

fn _get_display_value(column_type: &ColumnType, value: &Value) -> String {
    match column_type {
        ColumnType::Scalar { inner } => match inner {
            InnerColumnType::String { kind, .. } => match kind.as_str() {
                "channel" => format!("<#{}>", value.as_str().unwrap_or(&value.to_string())),
                "role" => format!("<@&{}>", value.as_str().unwrap_or(&value.to_string())),
                "user" => format!("<@{}>", value.as_str().unwrap_or(&value.to_string())),
                _ => {
                    let v = value
                        .as_str()
                        .unwrap_or(&value.to_string())
                        .replace("`", "\\`");

                    if v.len() > 1024 {
                        format!("```{}```", &v[..1021])
                    } else if v.contains('\n') {
                        format!("```\n{}```", v)
                    } else {
                        format!("``{}``", v)
                    }
                }
            },
            InnerColumnType::BitFlag { values } => {
                let v = match value {
                    Value::Number(v) => {
                        if let Some(v) = v.as_i64() {
                            v
                        } else {
                            return value.to_string();
                        }
                    }
                    Value::String(v) => {
                        if let Ok(v) = v.parse::<i64>() {
                            v
                        } else {
                            return v.to_string();
                        }
                    }
                    _ => return value.to_string(),
                };

                let mut result = Vec::new();
                for (name, flag) in values.iter() {
                    if v & *flag == *flag {
                        result.push(format!("`{}` ({})", name, flag));
                    }
                }
                result.join(", ")
            }
            _ => value.to_string(),
        },
        ColumnType::Array { inner } => {
            // Then the value must also be an array, check that or fallback to scalar _get_display_value
            match value {
                Value::Array(values) => values
                    .iter()
                    .map(|v| _get_display_value(&ColumnType::new_scalar(inner.clone()), v))
                    .collect::<Vec<String>>()
                    .join(", "),
                _ => _get_display_value(&ColumnType::new_scalar(inner.clone()), value),
            }
        }
    }
}

pub enum Src<'a> {
    Interaction(
        (
            &'a serenity::all::CommandInteraction,
            &'a serenity::all::Context,
            serenity::all::UserId,
        ),
    ),
    Message(
        (
            &'a serenity::all::Message,
            &'a serenity::all::Context,
            serenity::all::UserId,
        ),
    ),
}

pub enum SrcResponse<'a> {
    Message((serenity::all::Message, &'a serenity::all::Context)),
    Interaction(
        (
            &'a serenity::all::CommandInteraction,
            &'a serenity::all::Context,
        ),
    ),
}

impl<'a> SrcResponse<'a> {
    pub fn ctx(&self) -> &'a serenity::all::Context {
        match self {
            Self::Message((_, ctx)) => ctx,
            Self::Interaction((_, ctx)) => ctx,
        }
    }

    pub async fn into_message(&self) -> Result<serenity::all::Message, crate::Error> {
        match self {
            Self::Message((msg, _)) => Ok(msg.clone()),
            Self::Interaction((i, ctx)) => {
                let msg = i.get_response(&ctx.http).await?;

                Ok(msg)
            }
        }
    }
}

impl<'a> Src<'a> {
    pub fn ctx(&self) -> &'a serenity::all::Context {
        match self {
            Self::Interaction((_, ctx, _)) => ctx,
            Self::Message((_, ctx, _)) => ctx,
        }
    }

    pub fn author(&self) -> serenity::all::UserId {
        match self {
            Self::Interaction((_, _, author)) => *author,
            Self::Message((_, _, author)) => *author,
        }
    }

    pub async fn send_initial_response(
        &self,
        embed: serenity::all::CreateEmbed<'a>,
        action_row: Option<serenity::all::CreateActionRow<'a>>,
    ) -> Result<SrcResponse<'a>, crate::Error> {
        match self {
            Self::Interaction((interaction, ctx, _)) => {
                interaction
                    .create_response(&ctx.http, {
                        let cir = serenity::all::CreateInteractionResponse::Message({
                            let mut cir = serenity::all::CreateInteractionResponseMessage::new()
                                .ephemeral(true)
                                .embed(embed);

                            if let Some(action_row) = action_row {
                                cir = cir.components(vec![action_row]);
                            }

                            cir
                        });

                        cir
                    })
                    .await?;

                Ok(SrcResponse::Interaction((interaction, ctx)))
            }
            Self::Message((message, ctx, _)) => {
                let msg = message
                    .channel_id
                    .send_message(&ctx.http, {
                        let mut cim = CreateMessage::new().embed(embed);

                        if let Some(action_row) = action_row {
                            cim = cim.components(vec![action_row]);
                        }

                        cim
                    })
                    .await?;

                Ok(SrcResponse::Message((msg, ctx)))
            }
        }
    }
}

fn create_embed<'a, Data: Clone>(
    setting: &Setting<Data>,
    values: &'a [indexmap::IndexMap<String, Value>],
    index: usize,
    title: impl Fn() -> String,
) -> serenity::all::CreateEmbed<'a> {
    let mut embed = serenity::all::CreateEmbed::default();

    embed = embed.title((title)());

    for column in setting.columns.iter() {
        let Some(value) = values[index].get(column.id.as_str()) else {
            continue;
        };

        let mut display_value = _get_display_value(&column.column_type, value);

        if display_value.len() > 1024 {
            display_value = format!("{}...", &display_value[..1021]);
        }

        embed = embed.field(column.name.to_string(), display_value, true);
    }

    embed
}

/// Settings viewer code for serenity, sends an embed, all that stuff
pub async fn settings_viewer<Data: Clone>(
    src: Src<'_>,
    setting: &Setting<Data>,
    data: &Data,
    filters: indexmap::IndexMap<String, Value>, // The filters to apply
) -> Result<(), crate::Error> {
    fn create_action_row<'a>(index: usize, total: usize) -> serenity::all::CreateActionRow<'a> {
        serenity::all::CreateActionRow::Buttons(
            vec![
                serenity::all::CreateButton::new("previous")
                    .style(serenity::all::ButtonStyle::Primary)
                    .label("Previous")
                    .disabled(index == 0),
                serenity::all::CreateButton::new("next")
                    .style(serenity::all::ButtonStyle::Primary)
                    .label("Next")
                    .disabled(index >= total - 1),
                serenity::all::CreateButton::new("first")
                    .style(serenity::all::ButtonStyle::Primary)
                    .label("First")
                    .disabled(false),
                serenity::all::CreateButton::new("close")
                    .style(serenity::all::ButtonStyle::Danger)
                    .label("Close")
                    .disabled(false),
            ]
            .into(),
        )
    }

    if setting.operations.view.is_none() {
        return Err("Unsupported operation (View) for setting".into());
    };

    let values = settings_view(setting, data, filters)
        .await
        .map_err(|e| format!("Error fetching settings: {:?}", e))?;

    if values.is_empty() {
        return Ok(());
    }

    let total_count: usize = values.len();

    let mut index = 0;

    let msg = src
        .send_initial_response(
            create_embed(setting, &values, index, || {
                format!("{} ({} of {})", setting.name, index + 1, total_count)
            }),
            Some(create_action_row(index, total_count)),
        )
        .await?
        .into_message()
        .await?;

    let collector = msg
        .id
        .await_component_interactions(src.ctx().shard.clone())
        .author_id(src.author())
        .timeout(Duration::from_secs(180));

    let mut collect_stream = collector.stream();

    while let Some(item) = collect_stream.next().await {
        let item_id = item.data.custom_id.as_str();

        match item_id {
            "previous" => {
                index = index.saturating_sub(1);
            }
            "next" => {
                index = usize::min(index + 1, total_count - 1);
            }
            "first" => {
                index = 0;
            }
            "close" => {
                item.defer(&src.ctx().http).await?;
                item.delete_response(&src.ctx().http).await?;
                break;
            }
            _ => {}
        }

        item.defer(&src.ctx().http).await?;

        if index > total_count {
            index = total_count - 1;
        }

        item.edit_response(
            &src.ctx().http,
            serenity::all::EditInteractionResponse::new()
                .embed(create_embed(setting, &values, index, || {
                    format!("{} ({} of {})", setting.name, index + 1, total_count)
                }))
                .components(vec![create_action_row(index, total_count)]),
        )
        .await?;
    }

    Ok(())
}

/// Common settings creator for poise, sends an embed, all that stuff
pub async fn settings_creator<Data: Clone>(
    src: Src<'_>,
    setting: &Setting<Data>,
    data: &Data,
    fields: indexmap::IndexMap<String, Value>, // The filters to apply
) -> Result<(), crate::Error> {
    if setting.operations.create.is_none() {
        return Err("Unsupported operation (Create) for setting".into());
    };

    let value = settings_create(setting, data, fields)
        .await
        .map_err(|e| format!("Failed to create setting: {:?}", e))?;

    // Send message that we are creating the setting
    src.send_initial_response(
        create_embed(setting, &[value], 0, || format!("Created {}", setting.name)),
        None,
    )
    .await?;

    Ok(())
}

/// Common settings updater for poise, sends an embed, all that stuff
pub async fn settings_updater<Data: Clone>(
    src: Src<'_>,
    setting: &Setting<Data>,
    data: &Data,
    fields: indexmap::IndexMap<String, Value>,
) -> Result<(), crate::Error> {
    if setting.operations.update.is_none() {
        return Err("Unsupported operation (Update) for setting".into());
    };

    let value = settings_update(setting, data, fields)
        .await
        .map_err(|e| format!("Failed to update setting: {:?}", e))?;

    src.send_initial_response(
        create_embed(setting, &[value], 0, || format!("Updated {}", setting.name)),
        None,
    )
    .await?;

    Ok(())
}

/// Common settings deleter for poise, sends an embed, all that stuff
pub async fn settings_deleter<Data: Clone>(
    src: Src<'_>,
    setting: &Setting<Data>,
    data: &Data,
    fields: indexmap::IndexMap<String, Value>,
) -> Result<(), crate::Error> {
    if setting.operations.delete.is_none() {
        return Err("Unsupported operation (Delete) for setting".into());
    }

    let mut pkey_str = Vec::new();

    for column in setting.columns.iter() {
        if column.primary_key {
            if let Some(value) = fields.get(column.id.as_str()) {
                pkey_str.push(format!("{}: {}", column.name, value));
            }
        }
    }

    settings_delete(setting, data, fields)
        .await
        .map_err(|e| format!("Error deleting setting: {:?}", e))?;

    src.send_initial_response(
        serenity::all::CreateEmbed::new()
            .title(format!("Deleted {}", setting.name))
            .description(format!(
                "Deleted {}: {}",
                setting.name, pkey_str.join(", ")
            )),
        None,
    )
    .await?;

    Ok(())
}
