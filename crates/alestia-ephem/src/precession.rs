//! IAU P03 and Vondrak P03LP long-period precession models for Earth's ecliptic and equator
//! matching `celephem/precession.h` and `celephem/precession.cpp` (`FR-014`).

use core::f64::consts::TAU;

/// Ecliptic pole coordinates `(PA, QA)` relative to the fixed J2000.0 ecliptic, in arcseconds.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EclipticPole {
    /// Ecliptic pole `PA` coordinate (arcseconds).
    pub pa: f64,
    /// Ecliptic pole `QA` coordinate (arcseconds).
    pub qa: f64,
}

/// Angles relating the ecliptic of date to the J2000.0 ecliptic (arcseconds).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EclipticAngles {
    /// Inclination `piA` of the ecliptic of date to J2000.0 ecliptic (arcseconds).
    pub inclination_pi_a: f64,
    /// Angle `PiA` along J2000.0 ecliptic to the ascending node of ecliptic of date (arcseconds).
    pub asc_node_pi_a: f64,
}

/// General precession and obliquity of the ecliptic (arcseconds).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PrecessionAngles {
    /// General precession in longitude `pA` (arcseconds).
    pub general_prec_pa: f64,
    /// Obliquity `epsA` between ecliptic and mean equator of date (arcseconds).
    pub obliquity_eps_a: f64,
}

/// Equatorial precession angles `(zetaA, zA, thetaA)` for the P03 model (arcseconds).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EquatorialPrecessionAngles {
    /// Equatorial precession angle `zetaA` (arcseconds).
    pub zeta_a: f64,
    /// Equatorial precession angle `zA` (arcseconds).
    pub z_a: f64,
    /// Equatorial precession angle `thetaA` (arcseconds).
    pub theta_a: f64,
}

struct EclipticPrecessionTerm {
    p_cos: f64,
    q_cos: f64,
    p_sin: f64,
    q_sin: f64,
    period_centuries: f64,
}

const ECLIPTIC_PRECESSION_TERMS: [EclipticPrecessionTerm; 6] = [
    EclipticPrecessionTerm {
        p_cos: 486.230_527,
        q_cos: 2559.065_245,
        p_sin: -2578.462_809,
        q_sin: 485.116_645,
        period_centuries: 2308.98,
    },
    EclipticPrecessionTerm {
        p_cos: -963.825_784,
        q_cos: 247.582_718,
        p_sin: -237.405_076,
        q_sin: -971.375_498,
        period_centuries: 1831.25,
    },
    EclipticPrecessionTerm {
        p_cos: -1868.737_098,
        q_cos: -957.399_054,
        p_sin: 1007.593_090,
        q_sin: -1930.464_338,
        period_centuries: 687.52,
    },
    EclipticPrecessionTerm {
        p_cos: -1589.172_175,
        q_cos: 493.021_354,
        p_sin: -423.035_168,
        q_sin: -1634.905_683,
        period_centuries: 729.97,
    },
    EclipticPrecessionTerm {
        p_cos: 429.442_489,
        q_cos: -328.301_413,
        p_sin: 337.266_785,
        q_sin: 429.594_383,
        period_centuries: 492.21,
    },
    EclipticPrecessionTerm {
        p_cos: -2244.742_029,
        q_cos: -339.969_833,
        p_sin: 221.240_093,
        q_sin: -2131.745_072,
        period_centuries: 708.13,
    },
];

struct PrecessionTerm {
    prec_cos: f64,
    obl_cos: f64,
    prec_sin: f64,
    obl_sin: f64,
    period_centuries: f64,
}

const PRECESSION_TERMS: [PrecessionTerm; 10] = [
    PrecessionTerm {
        prec_cos: -6180.062_4,
        obl_cos: 807.904_635,
        prec_sin: -2434.845_716,
        obl_sin: -2056.455_197,
        period_centuries: 409.90,
    },
    PrecessionTerm {
        prec_cos: -2721.869_299,
        obl_cos: -177.959_383,
        prec_sin: 538.034_071,
        obl_sin: -912.727_303,
        period_centuries: 396.15,
    },
    PrecessionTerm {
        prec_cos: 1460.746_498,
        obl_cos: 371.942_696,
        prec_sin: -1245.689_351,
        obl_sin: 447.71,
        period_centuries: 536.91,
    },
    PrecessionTerm {
        prec_cos: -1838.488_899,
        obl_cos: -176.029_134,
        prec_sin: 529.220_775,
        obl_sin: -611.297_411,
        period_centuries: 402.90,
    },
    PrecessionTerm {
        prec_cos: 949.518_077,
        obl_cos: -89.154_03,
        prec_sin: 277.195_375,
        obl_sin: 315.900_626,
        period_centuries: 417.15,
    },
    PrecessionTerm {
        prec_cos: 32.701_46,
        obl_cos: -336.048_179,
        prec_sin: 945.979_71,
        obl_sin: 12.390_157,
        period_centuries: 288.92,
    },
    PrecessionTerm {
        prec_cos: 598.054_819,
        obl_cos: -17.415_73,
        prec_sin: -955.163_661,
        obl_sin: -15.922_155,
        period_centuries: 4042.97,
    },
    PrecessionTerm {
        prec_cos: -293.145_284,
        obl_cos: -28.084_479,
        prec_sin: 93.894_079,
        obl_sin: -102.870_153,
        period_centuries: 304.90,
    },
    PrecessionTerm {
        prec_cos: 66.354_942,
        obl_cos: 21.456_146,
        prec_sin: 0.671_968,
        obl_sin: 24.123_484,
        period_centuries: 281.46,
    },
    PrecessionTerm {
        prec_cos: 18.894_136,
        obl_cos: 30.917_011,
        prec_sin: -184.663_935,
        obl_sin: 2.512_708,
        period_centuries: 204.38,
    },
];

/// DE405 J2000.0 obliquity of the ecliptic in arcseconds.
const EPS0_ARCSEC: f64 = 84_381.408_89;

/// Ecliptic precession using the Vondrak (2006) long-period P03LP extension.
///
/// `centuries` is Julian centuries from J2000.0 (`(tdb - J2000) / 36525.0`).
#[must_use]
pub fn ecliptic_precession_p03_lp(centuries: f64) -> EclipticPole {
    let t2 = centuries * centuries;
    let t3 = t2 * centuries;

    let mut pa = 5750.804_069 + 0.194_831_1 * centuries - 0.000_167_39 * t2 - 4.8e-8 * t3;
    let mut qa = -1673.999_018 + 0.347_445_9 * centuries + 0.000_112_43 * t2 - 6.4e-8 * t3;

    for term in &ECLIPTIC_PRECESSION_TERMS {
        let theta = TAU * centuries / term.period_centuries;
        let (sin_t, cos_t) = theta.sin_cos();
        pa += term.p_cos * cos_t + term.p_sin * sin_t;
        qa += term.q_cos * cos_t + term.q_sin * sin_t;
    }

    EclipticPole { pa, qa }
}

/// General precession and obliquity using Vondrak (2006) long-period P03LP extension.
///
/// `centuries` is Julian centuries from J2000.0.
#[must_use]
pub fn prec_obliquity_p03_lp(centuries: f64) -> PrecessionAngles {
    let t2 = centuries * centuries;
    let t3 = t2 * centuries;

    let mut general_prec_pa =
        7907.295_95 + 5044.374_034 * centuries - 0.007_134_73 * t2 + 6e-9 * t3;
    let mut obliquity_eps_a = 83_973.876_448 - 0.042_589_9 * centuries - 0.000_001_13 * t2;

    for term in &PRECESSION_TERMS {
        let theta = TAU * centuries / term.period_centuries;
        let (sin_t, cos_t) = theta.sin_cos();
        general_prec_pa += term.prec_cos * cos_t + term.prec_sin * sin_t;
        obliquity_eps_a += term.obl_cos * cos_t + term.obl_sin * sin_t;
    }

    PrecessionAngles {
        general_prec_pa,
        obliquity_eps_a,
    }
}

/// Equatorial precession angles `zetaA`, `zA`, `thetaA` using the IAU 2006 / Capitaine P03 model.
#[must_use]
pub fn equatorial_precession_angles_p03(centuries: f64) -> EquatorialPrecessionAngles {
    let t2 = centuries * centuries;
    let t3 = t2 * centuries;
    let t4 = t3 * centuries;
    let t5 = t4 * centuries;

    EquatorialPrecessionAngles {
        zeta_a: 2.650_545 + 2306.083_227 * centuries + 0.298_849_9 * t2 + 0.018_018_28 * t3
            - 0.000_005_971 * t4
            - 0.000_000_317_3 * t5,
        z_a: -2.650_545 + 2306.077_181 * centuries + 1.092_734_8 * t2 + 0.018_268_37 * t3
            - 0.000_028_596 * t4
            - 0.000_000_290_4 * t5,
        theta_a: 2004.191_903 * centuries
            - 0.429_493_4 * t2
            - 0.041_822_64 * t3
            - 0.000_007_089 * t4
            - 0.000_000_127_4 * t5,
    }
}

/// Ecliptic pole coordinates `(PA, QA)` using the P03 precession model (arcseconds).
#[must_use]
pub fn ecliptic_precession_p03(centuries: f64) -> EclipticPole {
    let t2 = centuries * centuries;
    let t3 = t2 * centuries;
    let t4 = t3 * centuries;
    let t5 = t4 * centuries;

    EclipticPole {
        pa: 4.199_094 * centuries + 0.193_987_3 * t2 - 0.000_224_66 * t3 - 0.000_000_912 * t4
            + 0.000_000_012 * t5,
        qa: -46.811_015 * centuries + 0.051_028_3 * t2 + 0.000_524_13 * t3
            - 0.000_006_46 * t4
            - 0.000_000_017_2 * t5,
    }
}

/// Ecliptic orientation angles `(piA, PiA)` of date using the P03 precession model (arcseconds).
#[must_use]
pub fn ecliptic_precession_angles_p03(centuries: f64) -> EclipticAngles {
    let t2 = centuries * centuries;
    let t3 = t2 * centuries;
    let t4 = t3 * centuries;
    let t5 = t4 * centuries;

    EclipticAngles {
        inclination_pi_a: 46.998_973 * centuries - 0.033_492_6 * t2 - 0.000_125_59 * t3
            + 0.000_000_113 * t4
            - 0.000_000_002_2 * t5,
        asc_node_pi_a: 629_546.793_6 - 867.957_58 * centuries + 0.157_992 * t2
            - 0.000_537_1 * t3
            - 0.000_047_97 * t4
            + 0.000_000_072 * t5,
    }
}

/// General precession and obliquity using the standard polynomial P03 model (arcseconds).
#[must_use]
pub fn prec_obliquity_p03(centuries: f64) -> PrecessionAngles {
    let t2 = centuries * centuries;
    let t3 = t2 * centuries;
    let t4 = t3 * centuries;
    let t5 = t4 * centuries;

    PrecessionAngles {
        general_prec_pa: 5028.796_195 * centuries + 1.105_434_8 * t2 + 0.000_079_64 * t3
            - 0.000_023_857 * t4
            - 0.000_000_038_3 * t5,
        obliquity_eps_a: EPS0_ARCSEC - 46.836_769 * centuries - 0.000_183_1 * t2 + 0.002_003_4 * t3
            - 0.000_000_576 * t4
            - 0.000_000_043_4 * t5,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        EPS0_ARCSEC, ecliptic_precession_angles_p03, ecliptic_precession_p03,
        ecliptic_precession_p03_lp, equatorial_precession_angles_p03, prec_obliquity_p03,
        prec_obliquity_p03_lp,
    };

    #[test]
    fn p03_and_p03lp_match_j2000_reference_values() -> Result<(), String> {
        let p03 = prec_obliquity_p03(0.0);
        if (p03.general_prec_pa).abs() > 1e-12 || (p03.obliquity_eps_a - EPS0_ARCSEC).abs() > 1e-12
        {
            return Err(format!("Unexpected J2000 P03 obliquity: {p03:?}"));
        }

        let p03_lp = prec_obliquity_p03_lp(0.0);
        if (p03_lp.general_prec_pa).abs() > 0.1
            || (p03_lp.obliquity_eps_a - EPS0_ARCSEC).abs() > 0.1
        {
            return Err(format!("P03LP diverged from P03 at J2000: {p03_lp:?}"));
        }

        let pole = ecliptic_precession_p03(0.0);
        assert_eq!(pole.pa, 0.0);
        assert_eq!(pole.qa, 0.0);

        let pole_lp = ecliptic_precession_p03_lp(1.0);
        assert!(pole_lp.pa.is_finite() && pole_lp.qa.is_finite());

        let ecl = ecliptic_precession_angles_p03(1.0);
        let eq = equatorial_precession_angles_p03(1.0);
        if (ecl.inclination_pi_a - 46.965_35).abs() > 1e-3 || (eq.theta_a - 2003.72).abs() > 0.1 {
            return Err(format!("Unexpected P03 1-century angles: {ecl:?}, {eq:?}"));
        }
        Ok(())
    }
}
