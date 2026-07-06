use std::path::Path;

use omega_drive_gateway::provider::legacy_session::{
    LegacyChannelState, LegacyDcOption, LegacyPeerInfo, LegacySessionData, LegacySessionReader,
};

pub struct SqliteLegacySessionReader;

impl LegacySessionReader for SqliteLegacySessionReader {
    fn read_legacy_session(&self, path: &Path) -> Result<LegacySessionData, String> {
        let conn = rusqlite::Connection::open(path).map_err(|e| e.to_string())?;

        let home_dc: i32 = conn
            .query_row("SELECT dc_id FROM dc_home LIMIT 1", [], |row| row.get(0))
            .unwrap_or(0);

        let dc_options = {
            let mut stmt = conn
                .prepare("SELECT dc_id, ipv4, ipv6, auth_key FROM dc_option")
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |row| {
                    Ok(LegacyDcOption {
                        id: row.get(0)?,
                        ipv4: row.get::<_, String>(1)?,
                        ipv6: row.get::<_, String>(2)?,
                        auth_key: row.get(3)?,
                    })
                })
                .map_err(|e| e.to_string())?;
            let mut result = Vec::new();
            for row in rows {
                result.push(row.map_err(|e| e.to_string())?);
            }
            result
        };

        let peer_infos = {
            let mut stmt = conn
                .prepare("SELECT peer_id, hash, subtype FROM peer_info")
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |row| {
                    Ok(LegacyPeerInfo {
                        peer_id: row.get(0)?,
                        hash: row.get(1)?,
                        subtype: row.get(2)?,
                    })
                })
                .map_err(|e| e.to_string())?;
            let mut result = Vec::new();
            for row in rows {
                result.push(row.map_err(|e| e.to_string())?);
            }
            result
        };

        let (pts, qts, date, seq) = conn
            .query_row(
                "SELECT COALESCE(pts, 0), COALESCE(qts, 0), COALESCE(date, 0), COALESCE(seq, 0) FROM update_state LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap_or((0, 0, 0, 0));

        let channels = {
            let mut stmt = conn
                .prepare("SELECT peer_id, pts FROM channel_state")
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |row| {
                    Ok(LegacyChannelState {
                        id: row.get(0)?,
                        pts: row.get(1)?,
                    })
                })
                .map_err(|e| e.to_string())?;
            let mut result = Vec::new();
            for row in rows {
                result.push(row.map_err(|e| e.to_string())?);
            }
            result
        };

        Ok(LegacySessionData {
            home_dc,
            dc_options,
            peer_infos,
            pts,
            qts,
            date,
            seq,
            channels,
        })
    }
}
