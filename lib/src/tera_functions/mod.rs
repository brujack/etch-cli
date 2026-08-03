use tera::{Function, Result, Tera, Value};

#[derive(Debug)]
pub struct ReadFileContents;

impl Function for ReadFileContents {
    fn call(&self, args: &std::collections::HashMap<String, Value>) -> Result<Value> {
        match args.get("path") {
            Some(value) => match value.as_str() {
                Some(path) => match std::fs::read_to_string(path) {
                    Ok(content) => Ok(content.trim().into()),
                    Err(err) => Err(err.into()),
                },

                None => Err(format!(
                    "Path: '{value}'. Error: Cannot convert argument 'path' to str"
                )
                .into()),
            },

            None => Err("Argument 'path' not set".into()),
        }
    }
}

pub fn register_functions(tera: &mut Tera) {
    tera.register_function("read_file_contents", ReadFileContents);
}

#[cfg(test)]
mod test {
    use super::*;
    use std::io::Write;
    use tera::{Context, Tera};

    #[test]
    fn missing_path_arg_returns_error() {
        let mut tera = Tera::default();
        tera.register_function("read_file_contents", ReadFileContents);

        let template = "{{ read_file_contents() }}";
        let result = tera.render_str(template, &Context::new());
        assert!(result.is_err());
    }

    #[test]
    fn nonexistent_file_returns_error() {
        let mut tera = Tera::default();
        tera.register_function("read_file_contents", ReadFileContents);

        let template =
            "{{ read_file_contents(path=\"/nonexistent/path/that/does/not/exist.txt\") }}";
        let result = tera.render_str(template, &Context::new());
        assert!(result.is_err());
    }

    #[test]
    fn register_functions_adds_to_tera() {
        let mut tera = Tera::default();
        register_functions(&mut tera);

        // Creating a template that uses the function will work if registered
        let template = "{{ read_file_contents(path=\"/nonexistent\") }}";
        let result = tera.render_str(template, &Context::new());
        // Should fail because file doesn't exist, not because function is unregistered
        assert!(result.is_err());
    }

    #[test]
    fn can_read_from_file() -> anyhow::Result<()> {
        let mut tera = Tera::default();
        tera.register_function("read_file_contents", ReadFileContents);

        let mut file = tempfile::NamedTempFile::new()?;

        let file_content = r#"
FKBR
KUCI
SXOE

"#;

        write!(file.as_file_mut(), "{file_content}")?;

        let template = format!(
            "{{{{ read_file_contents(path=\"{}\") }}}}",
            file.path().display()
        );

        let content = tera.render_str(&template, &Context::new())?;

        let expected_file_content = r#"FKBR
KUCI
SXOE"#;

        assert_eq!(expected_file_content, content);

        Ok(())
    }
}
