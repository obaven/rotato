use rand::{Rng, distributions::Alphanumeric};

pub fn get_random_string(len: usize) -> String {
    rand::thread_rng().sample_iter(&Alphanumeric).take(len).map(char::from).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_random_string() {
        let s = get_random_string(10);
        assert_eq!(s.len(), 10);
        let s2 = get_random_string(10);
        assert_ne!(s, s2); // Extremely unlikely to collide
    }
}
