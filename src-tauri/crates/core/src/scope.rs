pub use omega_drive_gateway::core::scope::DriveScope;

pub fn parse_scope(scope: Option<&str>) -> Result<Option<DriveScope>, String> {
    scope
        .map(|value| {
            value
                .parse::<DriveScope>()
                .map_err(|_| format!("Invalid drive scope: {value}"))
        })
        .transpose()
}
