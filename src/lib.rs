use std::collections::BTreeSet;

#[cfg(feature = "display")]
use proc_macro2::Ident;

#[cfg(feature = "display")]
use quote::quote;

use syn::Variant;

/// Holds the format string with placeholders and the fields used for interpolation.
///
/// The default `ToTokens` implementation creates match arms for the `Display` trait.
/// You can also use the struct's fields directly to implement custom match arms
/// for other traits.

pub struct Interpolate<'a> {
    /// The variant for which the format string is being interpolated.
    pub variant: &'a Variant,

    /// The format string with placeholders processed:
    pub rewritten_text: String,

    /// Set of unique field names used in the format string.
    pub identifiers: BTreeSet<String>,
}

impl Interpolate<'_> {
    /// The format string with placeholders processed:
    /// - Named values: `{name}` remains as is
    /// - Positional values: `{n}` becomes `__n` where n is the index
    ///   (manually specified or auto-incremented)
    pub fn parse<'a>(fmt_text: impl AsRef<str>, variant: &'a Variant) -> Interpolate<'a> {
        let (rewritten_text, identifiers) = parse_internal(fmt_text);

        Interpolate {
            variant,
            rewritten_text,
            identifiers,
        }
    }
}

/// Parses the format string, extracts field names, and processes placeholders.
fn parse_internal(text: impl AsRef<str>) -> (String, BTreeSet<String>) {
    let mut chars = text.as_ref().chars().peekable();
    let (mut identifers, mut text, mut positional_index) = (BTreeSet::new(), String::new(), -1);

    while let Some(c) = chars.next() {
        if c != '{' {
            text.push(c);
            continue;
        }

        // If the next character is also a '{', then it's an escaped '{'
        if let Some('{') = chars.peek() {
            text.push_str("{{");
            chars.next();
            continue;
        }

        let (mut identifier, mut traits) = ("".to_string(), None);
        while let Some(c) = chars.next() {
            if c == ':' {
                // Extract trait specifier between ':' and '}'
                while let Some(c) = chars.peek() {
                    if *c == '}' {
                        break;
                    }

                    traits.get_or_insert("".to_string()).push(*c);
                    chars.next();
                }

                continue;
            }

            if c == '}' {
                // Handle positional values by auto-incrementing the index when no identifier is provided
                if identifier.is_empty() {
                    positional_index += 1;
                    identifier.push_str(&format!("__{}", positional_index));
                }

                if identifier.parse::<u8>().is_ok() {
                    identifier = format!("__{}", identifier);
                }

                let traits = traits.as_ref().map(|c| format!(":{c}")).unwrap_or_default();
                text.push_str(&format!("{{{}{}}}", &identifier, traits));
                identifers.insert(identifier.clone());
                break;
            }

            identifier.push(c);
        }
    }

    (text, identifers)
}

#[cfg(feature = "display")]
impl quote::ToTokens for Interpolate<'_> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let variant_name = &self.variant.ident;
        let interpolated_text = &self.rewritten_text;

        let mappings = match &self.variant.fields {
            syn::Fields::Unit => {
                quote! {
                    Self::#variant_name => write!(f, #interpolated_text),
                }
            }
            syn::Fields::Unnamed(fields) => {
                let fields = fields.unnamed.iter().collect::<Vec<_>>();
                let assignments = fields.iter().flat_map(|field| {
                    field
                        .ident
                        .as_ref()
                        .and_then(|ident| build_ident_assignment(ident, &self.identifiers))
                });

                let fields_ident = self
                    .identifiers
                    .iter()
                    .map(|ident| Ident::new(ident, proc_macro2::Span::call_site()));

                quote! {
                    Self::#variant_name(#(#fields_ident,)* ..) => write!(f, #interpolated_text, #(#assignments),*),
                }
            }
            syn::Fields::Named(fields) => {
                let fields = fields.named.iter().collect::<Vec<_>>();
                let fields_ident = fields.iter().flat_map(|field| &field.ident);

                quote! {
                    Self::#variant_name { #(#fields_ident,)* } => write!(f, #interpolated_text),
                }
            }
        };

        tokens.extend(mappings);
    }
}

#[cfg(feature = "display")]
/// Build the assignment for the field if it is used in the format string.
fn build_ident_assignment(
    ident: &Ident,
    used_fields: &BTreeSet<String>,
) -> Option<proc_macro2::TokenStream> {
    use quote::format_ident;

    // If the field is not present in the format string, then we don't need to interpolate it
    if !used_fields.contains(&ident.to_string()) {
        return None;
    }

    let ident = format_ident!("__{}", ident);
    Some(quote! { #ident = self.#ident })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use super::parse_internal;

    fn to_set<T: ToString>(values: &[T]) -> BTreeSet<String> {
        values.iter().map(|a| a.to_string()).collect()
    }

    #[test]
    fn test_named_placeholders() {
        // Single named placeholder
        assert_eq!(
            parse_internal("Hello, {name}!"),
            ("Hello, {name}!".to_string(), to_set(&["name"]))
        );

        // Multiple named placeholders
        assert_eq!(
            parse_internal("Hello, {name}! You are {age} years old."),
            ("Hello, {name}! You are {age} years old.".to_string(), 
             to_set(&["name", "age"]))
        );
    }

    #[test]
    fn test_positional_placeholders() {
        // Explicit positional placeholders
        assert_eq!(
            parse_internal("Hello, {0}! {1}"),
            ("Hello, {__0}! {__1}".to_string(), to_set(&["__0", "__1"]))
        );

        // Implicit positional placeholders
        assert_eq!(
            parse_internal("Hello, {}! {}"),
            ("Hello, {__0}! {__1}".to_string(), to_set(&["__0", "__1"]))
        );

        // Mixed explicit and implicit positional placeholders
        // Note: The current implementation reuses indices for the same position
        assert_eq!(
            parse_internal("{} {1} {0} {}"),
            ("{__0} {__1} {__0} {__1}".to_string(), 
             to_set(&["__0", "__1"]))
        );
    }

    #[test]
    fn test_mixed_named_and_positional() {
        assert_eq!(
            parse_internal("Hello, {}! My name is {name}. I'm {} years old."),
            (
                "Hello, {__0}! My name is {name}. I'm {__1} years old.".to_string(),
                to_set(&["__0", "name", "__1"])
            )
        );
    }

    #[test]
    fn test_format_specifiers() {
        // Debug format specifier
        assert_eq!(
            parse_internal("Debug: {value:?}"),
            ("Debug: {value:?}".to_string(), to_set(&["value"]))
        );

        // Hex format specifier
        assert_eq!(
            parse_internal("Hex: {value:x}"),
            ("Hex: {value:x}".to_string(), to_set(&["value"]))
        );

        // Multiple format specifiers
        assert_eq!(
            parse_internal("Number: {num:04x} {num:#x}"),
            ("Number: {num:04x} {num:#x}".to_string(), 
             to_set(&["num", "num"]))
        );
    }

    #[test]
    fn test_edge_cases() {
        // Empty string
        assert_eq!(
            parse_internal(""),
            ("".to_string(), BTreeSet::new())
        );

        // No placeholders
        assert_eq!(
            parse_internal("Just a regular string"),
            ("Just a regular string".to_string(), BTreeSet::new())
        );

        // Only placeholders
        assert_eq!(
            parse_internal("{}{name}{0}"),
            ("{__0}{name}{__0}".to_string(), 
             to_set(&["__0", "name", "__0"]))
        );

        // Escaped braces
        assert_eq!(
            parse_internal("{{escaped}} {{braces}} {name}"),
            ("{{escaped}} {{braces}} {name}".to_string(), 
             to_set(&["name"]))
        );
    }

    #[test]
    fn test_complex_combinations() {
        assert_eq!(
            parse_internal("User {name}: {age} years, {height:.2}m, ID: {:08x}"),
            (
                "User {name}: {age} years, {height:.2}m, ID: {__0:08x}".to_string(),
                to_set(&["name", "age", "height", "__0"])
            )
        );
    }
}
