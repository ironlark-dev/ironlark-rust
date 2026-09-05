//! What one half's `[declares.<half>]` section says about its hooks.
//!
//! A hook is a mod entry point. An engine hook's invocation belongs to the
//! engine, so the manifest writes its name and nothing else; an author hook is
//! declared with its rule, and the rule is read from the field that states it
//! rather than from a word repeating it.

use crate::manifest::key;
use syn::LitStr;
use toml::de::DeValue;

/// The one hook nobody declares. A half's section existing is the statement
/// that its `init` exists, so listing it says nothing twice.
pub(crate) const INIT: &str = "init";

/// The field an input hook states its rule with.
const BINDINGS: &str = "default-bindings";
/// The field every author hook carries.
const NAME: &str = "name";

/// A half's hooks, in the order the manifest writes them. That order is the
/// numbering: an author hook's id is its position among this half's author
/// hooks, which is how a dispatch arriving as a number reaches a function
/// without a resolve.
pub(crate) struct Half {
    hooks: Vec<Hook>,
}

/// One entry under `hooks`.
pub(crate) struct Hook {
    pub(crate) name: String,
    pub(crate) rule: Rule,
}

/// How a hook is invoked.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Rule {
    /// The engine's own: an event of its choosing, at a time of its choosing.
    Engine,
    /// A player's input edge, bound to the controls the entry names.
    Input,
}

impl Half {
    /// Reads the `hooks` value of one half's section. An absent key is a half
    /// that answers to `init` alone.
    pub(crate) fn read(lit: &LitStr, hooks: Option<&DeValue<'_>>, half: &str) -> syn::Result<Self> {
        let refuse = |cause: String| syn::Error::new(lit.span(), cause);
        let Some(value) = hooks else {
            return Ok(Self { hooks: Vec::new() });
        };
        let DeValue::Array(items) = value else {
            return Err(refuse(format!(
                "hooks under [declares.{half}] is an array: a name for an engine hook, \
                 `{{ {NAME} = \"...\", {BINDINGS} = [...] }}` for one of the mod's own"
            )));
        };

        let mut hooks = Vec::with_capacity(items.len());
        for item in items {
            let hook = match item.get_ref() {
                DeValue::String(name) => Hook {
                    name: name.to_string(),
                    rule: Rule::Engine,
                },
                DeValue::Table(entry) => {
                    for (written, _) in entry.iter() {
                        let written = written.get_ref();
                        if written == NAME || written == BINDINGS {
                            continue;
                        }
                        return Err(refuse(format!(
                            "`{written}` is no field of a hook under [declares.{half}]; a hook of \
                             the mod's own carries `{NAME}` and `{BINDINGS}`"
                        )));
                    }
                    let Some(DeValue::String(name)) = key(entry, NAME) else {
                        return Err(refuse(format!(
                            "a hook under [declares.{half}] written as a table names itself: \
                             `{{ {NAME} = \"...\", {BINDINGS} = [...] }}`"
                        )));
                    };
                    if key(entry, BINDINGS).is_none() {
                        return Err(refuse(format!(
                            "the hook `{name}` under [declares.{half}] states no rule; \
                             `{BINDINGS}` is the one rule a hook of the mod's own can carry today, \
                             and an engine hook is written as its bare name"
                        )));
                    }
                    Hook {
                        name: name.to_string(),
                        rule: Rule::Input,
                    }
                }
                _ => {
                    return Err(refuse(format!(
                        "an entry of hooks under [declares.{half}] is a name or a table: \
                         `\"on_tick\"` for an engine hook, \
                         `{{ {NAME} = \"...\", {BINDINGS} = [...] }}` for one of the mod's own"
                    )));
                }
            };
            if hook.name == INIT {
                return Err(refuse(format!(
                    "`{INIT}` is not declared anywhere: the [declares.{half}] section existing is \
                     the statement that this half and its `{INIT}` exist"
                )));
            }
            if hooks.iter().any(|held: &Hook| held.name == hook.name) {
                return Err(refuse(format!(
                    "hooks under [declares.{half}] names `{}` twice; a hook is one entry point",
                    hook.name
                )));
            }
            hooks.push(hook);
        }
        Ok(Self { hooks })
    }

    /// The hooks the engine invokes, as this half declares them.
    pub(crate) fn engine(&self) -> impl Iterator<Item = &Hook> {
        self.of(Rule::Engine)
    }

    /// The mod's own hooks, in the order that numbers them.
    pub(crate) fn author(&self) -> impl Iterator<Item = &Hook> {
        self.hooks.iter().filter(|hook| hook.rule != Rule::Engine)
    }

    fn of(&self, rule: Rule) -> impl Iterator<Item = &Hook> {
        self.hooks.iter().filter(move |hook| hook.rule == rule)
    }

    pub(crate) fn declares(&self, name: &str) -> Option<&Hook> {
        self.hooks.iter().find(|hook| hook.name == name)
    }
}
