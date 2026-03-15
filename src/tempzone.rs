
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TempZone {
    Cool,
    B50,
    B51,
    B52,
    B53,
    B54,
    B55,
    B56,
    B57,
    B58,
}

impl TempZone {
    pub fn reduction_percent(self) -> u32 {
        match self {
            TempZone::Cool => 0,
            TempZone::B50 => 10,
            TempZone::B51 => 15,
            TempZone::B52 => 20,
            TempZone::B53 => 25,
            TempZone::B54 => 30,
            TempZone::B55 => 35,
            TempZone::B56 => 45,
            TempZone::B57 => 55,
            TempZone::B58 => 75,
        }
    }
}

pub fn zone_with_hysteresis(temp_mc: i32, prev: TempZone) -> TempZone {
    let t50 = 50_000;
    let t51 = 51_000;
    let t52 = 52_000;
    let t53 = 53_000;
    let t54 = 54_000;
    let t55 = 55_000;
    let t56 = 56_000;
    let t57 = 57_000;
    let t58 = 58_000;
    let h = 500;

    match prev {
        TempZone::Cool => {
            if temp_mc >= t58 { TempZone::B58 }
            else if temp_mc >= t57 { TempZone::B57 }
            else if temp_mc >= t56 { TempZone::B56 }
            else if temp_mc >= t55 { TempZone::B55 }
            else if temp_mc >= t54 { TempZone::B54 }
            else if temp_mc >= t53 { TempZone::B53 }
            else if temp_mc >= t52 { TempZone::B52 }
            else if temp_mc >= t51 { TempZone::B51 }
            else if temp_mc >= t50 { TempZone::B50 }
            else { TempZone::Cool }
        }
        TempZone::B50 => {
            if temp_mc >= t58 { TempZone::B58 }
            else if temp_mc >= t57 { TempZone::B57 }
            else if temp_mc >= t56 { TempZone::B56 }
            else if temp_mc >= t55 { TempZone::B55 }
            else if temp_mc >= t54 { TempZone::B54 }
            else if temp_mc >= t53 { TempZone::B53 }
            else if temp_mc >= t52 { TempZone::B52 }
            else if temp_mc >= t51 { TempZone::B51 }
            else if temp_mc < t50 - h { TempZone::Cool }
            else { TempZone::B50 }
        }
        TempZone::B51 => {
            if temp_mc >= t58 { TempZone::B58 }
            else if temp_mc >= t57 { TempZone::B57 }
            else if temp_mc >= t56 { TempZone::B56 }
            else if temp_mc >= t55 { TempZone::B55 }
            else if temp_mc >= t54 { TempZone::B54 }
            else if temp_mc >= t53 { TempZone::B53 }
            else if temp_mc >= t52 { TempZone::B52 }
            else if temp_mc < t51 - h { TempZone::B50 }
            else { TempZone::B51 }
        }
        TempZone::B52 => {
            if temp_mc >= t58 { TempZone::B58 }
            else if temp_mc >= t57 { TempZone::B57 }
            else if temp_mc >= t56 { TempZone::B56 }
            else if temp_mc >= t55 { TempZone::B55 }
            else if temp_mc >= t54 { TempZone::B54 }
            else if temp_mc >= t53 { TempZone::B53 }
            else if temp_mc < t52 - h { TempZone::B51 }
            else { TempZone::B52 }
        }
        TempZone::B53 => {
            if temp_mc >= t58 { TempZone::B58 }
            else if temp_mc >= t57 { TempZone::B57 }
            else if temp_mc >= t56 { TempZone::B56 }
            else if temp_mc >= t55 { TempZone::B55 }
            else if temp_mc >= t54 { TempZone::B54 }
            else if temp_mc < t53 - h { TempZone::B52 }
            else { TempZone::B53 }
        }
        TempZone::B54 => {
            if temp_mc >= t58 { TempZone::B58 }
            else if temp_mc >= t57 { TempZone::B57 }
            else if temp_mc >= t56 { TempZone::B56 }
            else if temp_mc >= t55 { TempZone::B55 }
            else if temp_mc < t54 - h { TempZone::B53 }
            else { TempZone::B54 }
        }
        TempZone::B55 => {
            if temp_mc >= t58 { TempZone::B58 }
            else if temp_mc >= t57 { TempZone::B57 }
            else if temp_mc >= t56 { TempZone::B56 }
            else if temp_mc < t55 - h { TempZone::B54 }
            else { TempZone::B55 }
        }
        TempZone::B56 => {
            if temp_mc >= t58 { TempZone::B58 }
            else if temp_mc >= t57 { TempZone::B57 }
            else if temp_mc < t56 - h { TempZone::B55 }
            else { TempZone::B56 }
        }
        TempZone::B57 => {
            if temp_mc >= t58 { TempZone::B58 }
            else if temp_mc < t57 - h { TempZone::B56 }
            else { TempZone::B57 }
        }
        TempZone::B58 => {
            if temp_mc < t58 - h { TempZone::B57 }
            else { TempZone::B58 }
        }
    }
}
