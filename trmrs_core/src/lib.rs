pub fn hello_world() -> String {
    "Hello from TRMRS core!".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello_world() {
        let result = hello_world();
        assert_eq!(result, "Hello from TRMRS core!");
    }
}
