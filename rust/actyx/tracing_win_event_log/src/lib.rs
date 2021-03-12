#[cfg(windows)]
mod layer;
#[cfg(windows)]
pub fn layer(name: &str) -> std::io::Result<layer::Layer> {
    layer::Layer::new(name)
}
