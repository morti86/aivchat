#[macro_export]
macro_rules! make_enum {
    ($name:ident, [$op1:ident, $($opt:ident),*]) => {
        #[derive(Clone, Debug, Copy, PartialEq)]
        pub enum $name {
            $op1,
            $(
                $opt,
            )*
        }

        impl Default for $name {
            fn default() -> Self {
                $name::$op1
            }
        }

        impl $name {
            // Fixed array with commas
            pub const ALL: &'static [Self] = &[$name::$op1, $($name::$opt),+];

            pub fn to_string(&self) -> String {
                match self {
                    $name::$op1 => stringify!($op1).to_string(),
                    $(
                        $name::$opt => stringify!($opt).to_string(),
                    )*
                }
            }

            pub fn as_str(&self) -> &str {
                match self {
                    $name::$op1 => stringify!($op1),
                    $(
                        $name::$opt => stringify!($opt),
                    )*
                }
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                let s = s.as_str();
                match s {
                    stringify!($op1) => $name::$op1,
                    $(
                        stringify!($opt) => $name::$opt,
                    )*
                        _ => $name::$op1,
                }

            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str(self.to_string().as_str())
            }
        }
    };
}

pub fn substring_between<'a>(s: &'a str, delimiter: &str) -> Option<&'a str> {
    if delimiter.is_empty() {
        return None;
    }

    let delimiter_len = delimiter.len();
    let start = s.find(delimiter)?;  // Returns None if first delimiter not found
    let rest = &s[start + delimiter_len..];
    let end = rest.find(delimiter)?;  // Returns None if second delimiter not found

    Some(&rest[..end])
}
