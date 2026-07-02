use poise::serenity_prelude::Mentionable;
use songbird::CoreEvent;

use crate::voice_events::Receiver;

pub struct Data {} // User data, which is stored and accessible in all command invocations
type Context<'a> = poise::Context<'a, Data, anyhow::Error>;

#[poise::command(slash_command)]
pub async fn join(ctx: Context<'_>) -> anyhow::Result<()> {
    let Some(guild_id) = ctx.guild_id() else {
        ctx.reply("This command can only be used in a server")
            .await?;

        return Ok(());
    };

    let Some(channel_id) = guild_id
        .get_user_voice_state(ctx.http(), ctx.author().id)
        .await?
        .channel_id
    else {
        ctx.reply("You are not in a voice channel").await?;

        return Ok(());
    };

    let manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    // Some events relating to voice receive fire *while joining*.
    // We must make sure that any event handlers are installed before we attempt to join.
    {
        let handler_lock = manager.get_or_insert(guild_id);
        let mut handler = handler_lock.lock().await;

        let evt_receiver = Receiver::new();

        handler.add_global_event(CoreEvent::SpeakingStateUpdate.into(), evt_receiver.clone());
        handler.add_global_event(CoreEvent::ClientDisconnect.into(), evt_receiver.clone());
        handler.add_global_event(CoreEvent::VoiceTick.into(), evt_receiver);
    }

    if manager.join(guild_id, channel_id).await.is_ok() {
        ctx.reply(&format!("Joined {}", channel_id.mention()))
            .await?;
    } else {
        // Although we failed to join, we need to clear out existing event handlers on the call.
        _ = manager.remove(guild_id).await;

        ctx.reply("Error joining the channel").await?;
    }

    Ok(())
}

#[poise::command(slash_command)]
pub async fn leave(ctx: Context<'_>) -> anyhow::Result<()> {
    let Some(guild_id) = ctx.guild_id() else {
        ctx.reply("This command can only be used in a server")
            .await?;

        return Ok(());
    };

    let manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();
    let has_handler = manager.get(guild_id).is_some();

    if has_handler {
        if let Err(e) = manager.remove(guild_id).await {
            ctx.reply(format!("Failed: {:?}", e)).await?;
        }

        ctx.reply("Left voice channel").await?;
    } else {
        ctx.reply("Not in a voice channel").await?;
    }

    Ok(())
}
