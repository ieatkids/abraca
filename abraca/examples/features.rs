use abraca::{prelude::*, quant::FeatureCenter};
use chrono::NaiveDateTime;
use std::f64::consts::LN_2;

struct MidPx {
    inst: Inst,
    name: String,
    value: Option<f64>,
    update_time: NaiveDateTime,
}

impl MidPx {
    fn new(inst: &Inst) -> Self {
        Self {
            inst: inst.clone(),
            value: None,
            name: format!("MidPx_{}", inst.to_string()),
            update_time: NaiveDateTime::default(),
        }
    }
}

impl Feature for MidPx {
    fn name(&self) -> &str {
        &self.name
    }

    fn on_depth(&mut self, depth: &Depth) {
        let px = (depth.asks[0].0 + depth.bids[0].0) / 2.0;
        self.value = Some(px);
        self.update_time = depth.exch_time;
    }

    #[allow(unused_variables)]
    fn on_trade(&mut self, trade: &Trade) {}

    fn value(&self) -> Option<f64> {
        self.value
    }

    fn update_time(&self) -> NaiveDateTime {
        self.update_time
    }

    fn is_intrested(&self, inst: &Inst) -> bool {
        self.inst == *inst
    }
}

struct EmaPx {
    inst: Inst,
    halflife: i64,
    name: String,
    value: Option<f64>,
    update_time: NaiveDateTime,
}

impl EmaPx {
    fn new(inst: &Inst, halflife: i64) -> Self {
        Self {
            inst: inst.clone(),
            halflife,
            name: format!("EmaPx_{}_{}", inst.to_string(), halflife),
            value: None,
            update_time: NaiveDateTime::default(),
        }
    }
}

impl Feature for EmaPx {
    fn name(&self) -> &str {
        &self.name
    }

    fn is_intrested(&self, inst: &Inst) -> bool {
        self.inst == *inst
    }

    #[allow(unused_variables)]
    fn on_depth(&mut self, depth: &Depth) {}

    fn on_trade(&mut self, trade: &Trade) {
        if let Some(value) = self.value {
            let dt = (trade.exch_time - self.update_time).num_seconds();
            let w = (-LN_2 * dt as f64 / self.halflife as f64).exp();
            self.value = Some(w * value + (1.0 - w) * trade.px);
        } else {
            self.value = Some(trade.px);
        }
        self.update_time = trade.exch_time;
    }

    fn value(&self) -> Option<f64> {
        self.value
    }

    fn update_time(&self) -> NaiveDateTime {
        self.update_time
    }
}

struct MyFeatureLib;

impl FeatureLib for MyFeatureLib {
    fn name(&self) -> &str {
        "MyFeatureLib"
    }

    fn create_feature(&self, name: &str) -> Option<Box<dyn Feature>> {
        let mut parts = name.split('_');
        let Some(fname) = parts.next() else{
            return None;
        };
        let Some(inst) = parts.next() else{
            return None;
        };
        let Ok(inst) = Inst::try_from(inst) else{
            return None;
        };
        match fname {
            "MidPx" => return Some(Box::new(MidPx::new(&inst))),
            "EmaPx" => {
                let Some(halflife) = parts.next() else {
                    return  None;
                };
                let Ok(halflife) = halflife.parse::<i64>() else{
                    return None;
                  };
                return Some(Box::new(EmaPx::new(&inst, halflife)));
            }
            _ => return None,
        }
    }
}

fn main() {
    let mut center = FeatureCenter::new(MyFeatureLib);
    center.add_feature("MidPx_Okx.BTC.USDT.Spot");
    center.add_feature("EmaPx_Okx.BTC.USDT.Spot_10");
    center.add_feature("EmaPx_Okx.BTC.USDT.Spot_20");
    println!("{:?}", center.id_map);
}
