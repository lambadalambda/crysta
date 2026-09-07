//! Stateful browser adapter for the source-derived Pandora preview.

/// One authenticated, compiled Pandora preview session.
pub struct Session {
    preview: map_inspector::PandoraPreview,
}

impl Session {
    /// Authenticate and compile a Japanese ROM image entirely in memory.
    ///
    /// # Errors
    ///
    /// Returns the source validation or compilation error.
    pub fn new(bytes: &[u8]) -> Result<Self, String> {
        map_inspector::PandoraPreview::from_rom_bytes(bytes)
            .map(|preview| Self { preview })
            .map_err(|error| error.to_string())
    }

    /// Clear an existing slot before attempting replacement.
    ///
    /// # Errors
    ///
    /// Returns the source validation or compilation error. The slot remains
    /// empty on failure, preventing stale gameplay after malformed selection.
    pub fn replace(slot: &mut Option<Self>, bytes: &[u8]) -> Result<(), String> {
        *slot = None;
        let session = Self::new(bytes)?;
        *slot = Some(session);
        Ok(())
    }

    /// Return the current schema-versioned JSON state.
    #[must_use]
    pub fn state(&self) -> String {
        self.preview.state().to_string()
    }

    /// Apply one existing preview command and return the resulting JSON state.
    ///
    /// # Errors
    ///
    /// Rejects command bytes outside the closed `0..=10` protocol without
    /// changing the retained session.
    pub fn step(&mut self, input: u8) -> Result<String, String> {
        if input > 10 {
            return Err("unsupported preview command".into());
        }
        self.preview.step(input);
        Ok(self.state())
    }

    /// Apply a bounded parity fixture and return only its final JSON state.
    ///
    /// This is a qualification transport helper, not an interactive timing API.
    ///
    /// # Errors
    ///
    /// Rejects more than 6,000 commands or any byte outside `0..=10` before
    /// applying an input.
    pub fn run_parity_inputs(&mut self, inputs: &[u8]) -> Result<String, String> {
        if inputs.len() > 6_000 || inputs.iter().any(|input| *input > 10) {
            return Err("unsupported parity input fixture".into());
        }
        for &input in inputs {
            self.preview.step(input);
        }
        Ok(self.state())
    }

    /// Start the accepted New Game path and return its initial state.
    pub fn new_game(&mut self) -> String {
        self.preview.new_game();
        self.state()
    }

    /// Restore the accepted saved checkpoint and return its state.
    pub fn reset(&mut self) -> String {
        self.preview.reset();
        self.state()
    }

    /// Copy the immutable source-derived art manifest for host ownership.
    #[must_use]
    pub fn art(&self) -> Vec<u8> {
        self.preview.art().to_vec()
    }

    /// Copy one exact admitted BMP for host ownership.
    ///
    /// # Errors
    ///
    /// Rejects keys outside the closed six-background capability.
    pub fn background(&self, key: &str) -> Result<Vec<u8>, String> {
        let bytes = match key {
            "house" => Some(self.preview.bitmap()),
            "exterior" => Some(self.preview.exterior_bitmap()),
            "town13" | "cellars" | "box" | "tour" => self.preview.extra_bitmap(key),
            _ => None,
        };
        bytes
            .map(<[u8]>::to_vec)
            .ok_or_else(|| "unsupported preview background".into())
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::Session;
    use wasm_bindgen::prelude::*;

    /// JavaScript-owned stateful Pandora preview.
    #[wasm_bindgen(js_name = PandoraPreview)]
    pub struct WasmPandoraPreview {
        inner: Session,
    }

    #[wasm_bindgen(js_class = PandoraPreview)]
    impl WasmPandoraPreview {
        /// Authenticate and compile one browser-local ROM selection.
        #[wasm_bindgen(constructor)]
        pub fn new(bytes: &[u8]) -> Result<WasmPandoraPreview, String> {
            Session::new(bytes).map(|inner| Self { inner })
        }

        /// Return current JSON state.
        pub fn state(&self) -> String {
            self.inner.state()
        }

        /// Apply one bounded input command.
        pub fn step(&mut self, input: u8) -> Result<String, String> {
            self.inner.step(input)
        }

        /// Apply one bounded qualification fixture without interactive transport.
        #[wasm_bindgen(js_name = runParityInputs)]
        pub fn run_parity_inputs(&mut self, inputs: &[u8]) -> Result<String, String> {
            self.inner.run_parity_inputs(inputs)
        }

        /// Start New Game.
        pub fn new_game(&mut self) -> String {
            self.inner.new_game()
        }

        /// Reset to the saved checkpoint.
        pub fn reset(&mut self) -> String {
            self.inner.reset()
        }

        /// Return an owned art-manifest copy.
        pub fn art(&self) -> Vec<u8> {
            self.inner.art()
        }

        /// Return an owned admitted-background copy.
        pub fn background(&self, key: &str) -> Result<Vec<u8>, String> {
            self.inner.background(key)
        }
    }
}
