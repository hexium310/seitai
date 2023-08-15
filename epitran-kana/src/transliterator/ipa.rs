use anyhow::{Result, Error};
use pyo3::marker::Python;

pub(crate) fn transliterate(word: &str) -> Result<String> {
    Python::with_gil(|py| {
        let epitran = py.import("epitran")?.getattr("Epitran")?;
        let transliterator = epitran.call1(("eng-Latn", ))?;
        let ipa = transliterator.getattr("transliterate")?.call1((word, ))?.extract::<String>().map_err(Error::msg)?;
        Ok(ipa.to_string())
    })
}
