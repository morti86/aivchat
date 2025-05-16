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

#[cfg(target_os = "windows")]
pub fn open_link(url: &str) {
    extern crate shell32;
    extern crate winapi;

    use std::ffi::CString;
    use std::ptr;

    unsafe {
        shell32::ShellExecuteA(ptr::null_mut(),
                               CString::new("open").unwrap().as_ptr(),
                               CString::new(url.replace("\n", "%0A")).unwrap().as_ptr(),
                               ptr::null(),
                               ptr::null(),
                               winapi::SW_SHOWNORMAL);
    }
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

#[target_feature(enable = "avx2")]
#[cfg(target_arch = "x86_64")]
pub fn level_avx2(slice: &[f32]) -> f32 {
    use std::arch::x86_64::*;
    let (prefix, middle, tail) = unsafe { slice.align_to::<__m256>() };
    let mut sum = prefix.iter().sum::<f32>();
    sum += tail.iter().fold(0.0, |acc, &e| acc + f32::powi(e,2));

    let mut base = _mm256_setzero_ps();
    for e in middle.iter() {
        let t = _mm256_mul_ps(*e, *e);
        base = _mm256_add_ps(base, t);
    }

    let base: [f32; 8] = unsafe { std::mem::transmute(base) };
    sum += base.iter().fold(0.0, |acc,&e| acc + f32::powi(e,2));

    sum
}
