
pub fn should_rotate_by_policy(notes: &str, rotation_days: u64) -> bool {
    // 1. Extract Last Rotated line
    if let Some(line) = notes.lines().find(|l| l.starts_with("Last Rotated: ")) {
         let date_str = line.trim_start_matches("Last Rotated: ").trim();
         if let Ok(last_rotated) = chrono::DateTime::parse_from_rfc3339(date_str) {
             let now = chrono::Utc::now();
             let msg_age = now.signed_duration_since(last_rotated);
             if msg_age.num_days() < rotation_days as i64 {
                 println!("    Skipping: Last rotated {} days ago (Policy: {} days)", msg_age.num_days(), rotation_days);
                 return false;
             }
         } else {
             println!("    Warning: Could not parse Last Rotated date '{}'. Rotating safely.", date_str);
         }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_rotate_policy() {
        // No last rotated = true
        assert!(should_rotate_by_policy("", 30));
        
        // Old date = true
        let old_date = "Last Rotated: 2020-01-01T00:00:00+00:00\nNotes...";
        assert!(should_rotate_by_policy(old_date, 30));
        
        // Brand new date = false
        let now = chrono::Utc::now().to_rfc3339();
        let new_notes = format!("Last Rotated: {}\nNotes...", now);
        assert!(!should_rotate_by_policy(&new_notes, 30));

        // Invalid date = true (fail safe)
        assert!(should_rotate_by_policy("Last Rotated: invalid-date\nNotes...", 30));
    }
}
