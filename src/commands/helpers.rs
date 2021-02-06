use serenity::client::Context;
use serenity::model::prelude::*;

pub async fn get_text_channel(
    ctx: &Context,
    server_id: &GuildId,
    target: &str,
) -> Option<GuildChannel> {
    let channels = server_id.channels(&ctx.http).await.unwrap();
    channels
        .values()
        .find(
            |server_channel| match (server_channel.kind, &server_channel.name) {
                (ChannelType::Text, name) if name == target => true,
                _ => false,
            },
        )
        .map(|gc| gc.clone())
}
