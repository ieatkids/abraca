use std::collections::HashMap;

use crate::prelude::*;

pub struct FeatureCenter<F: FeatureLib> {
    pub lib: F,
    pub features: Vec<Box<dyn Feature>>,
    pub id_map: HashMap<String, usize>,
}

impl<F: FeatureLib> FeatureCenter<F> {
    pub fn new(lib: F) -> Self {
        Self {
            lib,
            features: Vec::new(),
            id_map: HashMap::new(),
        }
    }

    pub fn add_feature(&mut self, name: &str) {
        if self.id_map.contains_key(name) {
            return;
        }

        if let Some(feature) = self.lib.create_feature(name) {
            self.features.push(feature);
            self.id_map.insert(name.to_owned(), self.features.len() - 1);
        }
    }

    pub fn values(&self) -> Vec<Option<f64>> {
        self.features.iter().map(|f| f.value()).collect()
    }

    pub fn on_depth(&mut self, depth: &Depth) {
        self.features
            .iter_mut()
            .filter(|f| f.is_intrested(&depth.inst))
            .for_each(|f| f.on_depth(depth));
    }

    pub fn on_trade(&mut self, trade: &Trade) {
        self.features
            .iter_mut()
            .filter(|f| f.is_intrested(&trade.inst))
            .for_each(|f| f.on_trade(trade));
    }
}
