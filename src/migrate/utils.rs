pub(crate) fn short_uuid(uuid: &str) -> &str {
    uuid.get(..8).unwrap_or(uuid)
}
