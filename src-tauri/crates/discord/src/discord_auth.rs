use std::result::Result;

use omega_drive_gateway::core::types::ReachableDestination;
use serenity::http::{GuildPagination, Http as DiscordHttp};
use serenity::model::id::GuildId;

pub async fn list_reachable_discord_guilds(
    token: &str,
) -> Result<Vec<ReachableDestination>, anyhow::Error> {
    let http = DiscordHttp::new(token);
    let mut after: Option<GuildId> = None;
    let mut guilds = Vec::new();

    loop {
        let batch = http
            .get_guilds(after.map(GuildPagination::After), Some(200))
            .await?;
        if batch.is_empty() {
            break;
        }

        let batch_len = batch.len();
        for guild in &batch {
            guilds.push(ReachableDestination {
                id: guild.id.get().to_string(),
                name: guild.name.clone(),
            });
        }

        if batch_len < 200 {
            break;
        }
        after = batch.last().map(|guild| guild.id);
    }

    guilds.sort_by(|left, right| {
        left.name
            .to_ascii_lowercase()
            .cmp(&right.name.to_ascii_lowercase())
            .then_with(|| left.id.cmp(&right.id))
    });
    guilds.dedup_by(|left, right| left.id == right.id);

    Ok(guilds)
}

pub async fn validate_discord_token(token: &str) -> Result<(), anyhow::Error> {
    let http = DiscordHttp::new(token);
    http.get_current_user().await?;
    Ok(())
}
