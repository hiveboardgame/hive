include!("test_db_support.rs");

#[cfg(test)]
mod tests {
    use super::is_test_database_name;

    #[test]
    fn test_database_names_must_identify_the_database_itself_as_test() {
        assert!(is_test_database_name("test"));
        assert!(is_test_database_name("test_hive"));
        assert!(is_test_database_name("hive-test"));
        assert!(is_test_database_name("hive_test_db"));

        assert!(!is_test_database_name("hive"));
        assert!(!is_test_database_name("contest"));
    }
}
