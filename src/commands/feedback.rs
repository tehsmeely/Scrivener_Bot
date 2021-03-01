use serenity::prelude::Context;
use serenity::model::prelude::Message;
use serenity::framework::standard::{Args, CommandResult};
use serenity::framework::standard::macros::command;
use std::str::FromStr;
use serenity::model::id::UserId;
use crate::config::GeneralAppConfigData;
use serenity::utils::MessageBuilder;
use crate::utils::trait_extensions::MessageBuilderExt;
use serenity::model::user::User;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

#[derive(Debug, EnumIter)]
enum FeedbackKind {
    Bug,
    Feature,
    Misc
}
impl FeedbackKind{
    fn to_str(&self) -> &'static str {
        match self {
            Self::Bug=> "bug",
            Self::Feature => "feature",
            Self::Misc => "misc",
        }
    }
}
impl FromStr for FeedbackKind {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "bug" => Ok(Self::Bug),
            "feature" => Ok(Self::Feature),
            "misc" => Ok(Self::Misc),
            _ => Err(format!("Invalid FeedbackKind name {}", s)),
        }
    }
}

#[command("feedback")]
#[usage("<kind> <your feedback>")]
#[description("Give you feedback on the bot which will be given to the bot admin/developer. Kind can be one of: bug|feature|misc")]
#[example("feature It would be really cool if it shot lasers")]
#[example("bug When i ask for a bunny wordcloud it dies")]
async fn feedback(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let invalid_arg_respose = String::from("Invalid arguments, try [help feedback]");
    let reply = match args.len() {
        0 | 1 => String::from("Expecting 2+ arguments: <feedback kind> <feedback message>"),
        _ => match args.single::<FeedbackKind>() {
            Ok(feedback_kind) => {
                let from_ = {
                    let maybe_server =
                        if let Some(guild_id) = msg.guild_id {
                            if let Some(guild_name) = guild_id.name(ctx).await {
                                format!(" on {}", guild_name)
                            } else {
                                String::from("")
                            }
                        } else {
                            String::from("")
                        };
                    format!("{}{}",
                    msg.author.name, maybe_server)
                };
                send_feedback(ctx, feedback_kind, args.rest(), &from_).await
            }
            Err(e) => invalid_arg_respose
        }
    };
    msg.reply(ctx, reply).await?;
    Ok(())
}

async fn send_feedback(
    ctx: &Context,
    feedback_kind: FeedbackKind,
    feedback: &str,
    from_: &str
) -> String {
    let feedback_receiver = {
        let config_lock = {
            let data_read = ctx.data.read().await;
            data_read
                .get::<GeneralAppConfigData>()
                .expect("Expected GeneralAppConfigData in TypeMap.")
                .clone()
        };
        let bot_admin = config_lock.read().unwrap().bot_admin.clone();
        bot_admin
    };
    let feedback_receiver: Option<User> = match feedback_receiver {
        Some(user_id) =>
            match user_id.to_user(ctx).await {
                Ok(user) => Some(user),
                Err(_) => None
            },
        None => None
    };
    match feedback_receiver {
        Some(user) => {
            let content = MessageBuilder::new()
                .push(format!("You received feedback from {}", from_)).newline().push(format!("Kind: {}", feedback_kind.to_str())).newline().push(feedback).build();
            user.direct_message(ctx, |m|
                m.content(content)
            ).await.unwrap();
            String::from("Thanks for your feedback. It has been sent to a bot admin")
        }
        None => {
            String::from("No bot admin has been configured so I can't submit your feedback, sorry 😞")
        }
    }

}
#[test]
fn usage_matches_all_masks() {
    let desc: &str = FEEDBACK_COMMAND_OPTIONS.desc.unwrap();
    let match_ = "Kind can be one of: ";
    let kind_index = desc.rfind(match_).unwrap();
    let (_, kinds_from_desc) = desc.split_at(kind_index + match_.len());
    let all_kinds_from_enum_iter: String = FeedbackKind::iter()
        .map(|fbk| String::from(fbk.to_str()))
        .collect::<Vec<String>>()
        .join("|");
    assert_eq!(all_kinds_from_enum_iter, kinds_from_desc);
}
