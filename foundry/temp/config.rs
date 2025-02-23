use gkr::executor::{M31ExtConfigSha2, BN254ConfigMIMC5, GF2ExtConfigSha2};

pub trait Config {
    type DefaultGKRConfig;
    type DefaultGKRFieldConfig;
}

pub struct BN254Config;

impl Config for BN254Config {
    type DefaultGKRConfig = M31ExtConfigSha2;
    type DefaultGKRFieldConfig = BN254ConfigMIMC5;
}

pub struct GF2Config;

impl Config for GF2Config {
    type DefaultGKRConfig = GF2ExtConfigSha2;
    type DefaultGKRFieldConfig = BN254ConfigMIMC5;
}