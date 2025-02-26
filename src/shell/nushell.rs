use std::{fmt::Display, path::Path};

use crate::shell::Shell;

#[derive(Default)]
pub struct Nushell {}

enum EnvOp<'a> {
    Set { key: &'a str, val: &'a str },
    Hide { key: &'a str },
}

impl<'a> Display for EnvOp<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[allow(clippy::write_with_newline)]
        match self {
            EnvOp::Set { key, val } => write!(f, "set,{key},{val}\n"),
            EnvOp::Hide { key } => write!(f, "hide,{key},\n"),
        }
    }
}

impl Shell for Nushell {
    fn activate(&self, exe: &Path, flags: String) -> String {
        let exe = exe.display();

        formatdoc! {r#"
          export-env {{
            $env.MISE_SHELL = "nu"
            
            $env.config = ($env.config | upsert hooks {{
                pre_prompt: ($env.config.hooks.pre_prompt ++
                [{{
                condition: {{|| "MISE_SHELL" in $env }}
                code: {{|| mise_hook }}
                }}])
                env_change: {{
                    PWD: ($env.config.hooks.env_change.PWD ++
                    [{{
                    condition: {{|| "MISE_SHELL" in $env }}
                    code: {{|| mise_hook }}
                    }}])
                }}
            }})
          }}
            
          def "parse vars" [] {{
            $in | lines | parse "{{op}},{{name}},{{value}}"
          }}
            
          def --wrapped mise [command?: string, --help, ...rest: string] {{
            let commands = ["shell", "deactivate"]
            
            if ($command == null) {{
              ^"{exe}"
            }} else if ($command == "activate") {{
              $env.MISE_SHELL = "nu"
            }} else if ($command in $commands) {{
              ^"{exe}" $command ...$rest
              | parse vars
              | update-env
            }} else {{
              ^"{exe}" $command ...$rest
            }}
          }}
            
          def --env "update-env" [] {{
            for $var in $in {{
              if $var.op == "set" {{
                load-env {{($var.name): $var.value}}
              }} else if $var.op == "hide" {{
                hide-env $var.name
              }}
            }}
          }}

          def --env mise_hook [] {{
            ^"{exe}" hook-env{flags} -s nu
              | parse vars
              | update-env
          }}

        "#}
    }

    fn deactivate(&self) -> String {
        self.unset_env("MISE_SHELL")
    }

    fn set_env(&self, k: &str, v: &str) -> String {
        let k = shell_escape::unix::escape(k.into());
        let v = shell_escape::unix::escape(v.into());
        let v = v.replace("\\n", "\n");
        let v = v.replace('\'', "");

        EnvOp::Set { key: &k, val: &v }.to_string()
    }

    fn prepend_env(&self, k: &str, v: &str) -> String {
        format!("$env.{k} = ($env.{k} | prepend '{v}')\n")
    }

    fn unset_env(&self, k: &str) -> String {
        let k = shell_escape::unix::escape(k.into());
        EnvOp::Hide { key: k.as_ref() }.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::replace_path;

    #[test]
    fn test_hook_init() {
        let nushell = Nushell::default();
        let exe = Path::new("/some/dir/mise");
        assert_snapshot!(nushell.activate(exe, " --status".into()));
    }

    #[test]
    fn test_set_env() {
        assert_snapshot!(Nushell::default().set_env("FOO", "1"));
    }

    #[test]
    fn test_prepend_env() {
        let sh = Nushell::default();
        assert_snapshot!(replace_path(&sh.prepend_env("PATH", "/some/dir:/2/dir")));
    }

    #[test]
    fn test_unset_env() {
        assert_snapshot!(Nushell::default().unset_env("FOO"));
    }

    #[test]
    fn test_deactivate() {
        let deactivate = Nushell::default().deactivate();
        assert_snapshot!(replace_path(&deactivate));
    }
}
