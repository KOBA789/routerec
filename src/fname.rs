pub struct Filename([u8; 12]);

impl Filename {
    pub fn fileno_mut(&mut self) -> &mut [u8] {
        &mut self.0[2..8]
    }

    pub fn increment(&mut self) {
        increment_digits(self.fileno_mut())
    }

    pub fn into_inner(self) -> [u8; 12] {
        self.0
    }
}

impl Default for Filename {
    fn default() -> Self {
        let mut name = [0u8; 12];
        name.copy_from_slice(b"RR000000.TXT");
        Self(name)
    }
}

fn increment_digits(digits: &mut [u8]) {
    for digit in digits.iter_mut().rev() {
        if *digit == b'9' {
            *digit = b'0';
        } else {
            *digit += 1;
            return;
        }
    }
}

fn is_number(s: &[u8]) -> bool {
    s.iter().all(|b| (b'0'..=b'9').contains(b))
}

pub struct Builder {
    file_name: Filename,
}

impl Builder {
    pub fn new() -> Self {
        Self {
            file_name: Filename::default(),
        }
    }

    pub fn push(&mut self, basename: &[u8], extension: &[u8]) -> bool {
        if extension != b"TXT" {
            return false;
        }
        if basename.len() != 8 {
            return false;
        }
        let (prefix, new_fileno) = basename.split_at(2);
        if prefix != b"RR" {
            return false;
        }
        let fileno = self.file_name.fileno_mut();
        if !is_number(new_fileno) {
            return false;
        }
        if new_fileno <= &*fileno {
            return false;
        }
        fileno.copy_from_slice(new_fileno);
        true
    }

    pub fn finish(mut self) -> Filename {
        self.file_name.increment();
        self.file_name
    }
}
