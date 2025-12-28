
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TempZone {
    Cool,
    Z100,
    Z110,
    Z120,
    Z130,
}

impl TempZone {
    pub fn reduction_percent(self) -> u32 {
        match self {
            TempZone::Cool => 0,
            TempZone::Z100 => 10,
            TempZone::Z110 => 15,
            TempZone::Z120 => 25,
            TempZone::Z130 => 40,
        }
    }
}

pub fn zone_with_hysteresis(temp_mc: i32, prev: TempZone) -> TempZone {
    let t100 = 100_000;
    let t110 = 110_000;
    let t120 = 120_000;
    let t130 = 130_000;
    let h = 2_000;

    match prev {
        TempZone::Cool => {
            if temp_mc >= t130 { TempZone::Z130 }
            else if temp_mc >= t120 { TempZone::Z120 }
            else if temp_mc >= t110 { TempZone::Z110 }
            else if temp_mc >= t100 { TempZone::Z100 }
            else { TempZone::Cool }
        }
        TempZone::Z100 => {
            if temp_mc >= t130 { TempZone::Z130 }
            else if temp_mc >= t120 { TempZone::Z120 }
            else if temp_mc >= t110 { TempZone::Z110 }
            else if temp_mc < t100 - h { TempZone::Cool }
            else { TempZone::Z100 }
        }
        TempZone::Z110 => {
            if temp_mc >= t130 { TempZone::Z130 }
            else if temp_mc >= t120 { TempZone::Z120 }
            else if temp_mc < t110 - h { TempZone::Z100 }
            else { TempZone::Z110 }
        }
        TempZone::Z120 => {
            if temp_mc >= t130 { TempZone::Z130 }
            else if temp_mc < t120 - h { TempZone::Z110 }
            else { TempZone::Z120 }
        }
        TempZone::Z130 => {
            if temp_mc < t130 - h { TempZone::Z120 }
            else { TempZone::Z130 }
        }
    }
}
