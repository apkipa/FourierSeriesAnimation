use num::{traits::NumOps, Complex, Float, Num};
use std::fmt::Debug;
use std::{
    iter::Sum,
    ops::{Add, Mul, RangeInclusive},
};

pub trait SqrAbs {
    fn sqr_abs(&self) -> f64;
}

impl SqrAbs for f64 {
    fn sqr_abs(&self) -> f64 {
        self.powi(2)
    }
}

impl SqrAbs for f32 {
    fn sqr_abs(&self) -> f64 {
        self.powi(2).into()
    }
}

impl<T: SqrAbs + Add> SqrAbs for Complex<T> {
    fn sqr_abs(&self) -> f64 {
        self.re.sqr_abs() + self.im.sqr_abs()
    }
}

#[derive(Debug)]
pub struct FourierSeriesDesc<T: Float> {
    // Contract: coefficients.len() % 2 != 0
    coefficients: Vec<Complex<T>>,
}

// impl<T: Float> Index<isize> for FourierSeriesDesc<T> {
//     type Output = Complex<T>;

//     // Panics: If the vec has incorrect len or index is out of range, the function panics
//     // Index range is [-(n - 1) / 2, (n - 1) / 2]
//     fn index(&self, index: isize) -> &Self::Output {
//         let Self { coefficients } = self;
//         let half_range = ((coefficients.len() - 1) / 2) as isize;
//         assert!((-half_range..=half_range).contains(&index));
//         &coefficients[(index + half_range) as usize]
//     }
// }

impl<T: Float> FourierSeriesDesc<T>
where
    T: Mul<f64, Output = T>,
{
    pub fn as_vec(&self) -> &Vec<Complex<T>> {
        &self.coefficients
    }

    pub fn as_fn(&self) -> impl Fn(T) -> Complex<T> + '_ {
        let Self { coefficients } = self;
        let n = coefficients.len();
        move |t| {
            let half_range = ((n - 1) / 2) as isize;
            coefficients
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let i = i as isize - half_range;
                    *c * Complex::new(T::zero(), t * i as f64 * 2.0 * std::f64::consts::PI).exp()
                })
                .sum()
        }
    }
}

const X_N_16: usize = 16;
const X_POSITIONS_16: [f64; X_N_16] = [
    -0.989400934991649932596,
    -0.944575023073232576078,
    -0.865631202387831743880,
    -0.755404408355003033895,
    -0.617876244402643748447,
    -0.458016777657227386342,
    -0.281603550779258913230,
    -0.0950125098376374401853,
    0.0950125098376374401853,
    0.281603550779258913230,
    0.458016777657227386342,
    0.617876244402643748447,
    0.755404408355003033895,
    0.865631202387831743880,
    0.944575023073232576078,
    0.989400934991649932596,
];
const X_WEIGHTS_16: [f64; X_N_16] = [
    0.0271524594117540948518,
    0.0622535239386478928628,
    0.0951585116824927848099,
    0.124628971255533872052,
    0.149595988816576732082,
    0.169156519395002538189,
    0.182603415044923588867,
    0.189450610455068496285,
    0.189450610455068496285,
    0.182603415044923588867,
    0.169156519395002538189,
    0.149595988816576732082,
    0.124628971255533872052,
    0.0951585116824927848099,
    0.0622535239386478928628,
    0.0271524594117540948518,
];

const TOL: f64 = 1e-5;

// Ordinary quadrature
pub fn integrate<In: Num + Clone, Out: Num + Clone>(
    range: RangeInclusive<In>,
    func: impl Fn(In) -> Out,
) -> Out
where
    In: Mul<f64, Output = In>,
    Out: Mul<f64, Output = Out> + Mul<In, Output = Out> + Sum,
{
    let in_two = In::one() + In::one();
    let (start, end) = (range.start().clone(), range.end().clone());
    let half_length = (end.clone() - start.clone()) / in_two.clone();
    let middle = (start + end) / in_two;
    let result: Out = (0..X_N_16)
        .map(|n| func(middle.clone() + half_length.clone() * X_POSITIONS_16[n]) * X_WEIGHTS_16[n])
        .sum();
    result * half_length
}

// Adaptive quadrature
pub fn integrate_v2<In: Num + Clone, Out: Num + Clone>(
    range: RangeInclusive<In>,
    func: impl Fn(In) -> Out + Clone,
) -> Out
where
    In: Mul<f64, Output = In>,
    Out: Mul<f64, Output = Out> + Mul<In, Output = Out> + Sum + SqrAbs,
{
    fn inner<In: Num + Clone, Out: Num + Clone>(
        range: RangeInclusive<In>,
        func: impl Fn(In) -> Out + Clone,
        last_res: Out,
        avail_depth: usize,
    ) -> Out
    where
        In: Mul<f64, Output = In>,
        Out: Mul<f64, Output = Out> + Mul<In, Output = Out> + Sum + SqrAbs,
    {
        let in_two = In::one() + In::one();
        let (start, end) = (range.start().clone(), range.end().clone());
        let middle = (start.clone() + end.clone()) / in_two;
        let range_l = start.clone()..=middle.clone();
        let range_r = middle.clone()..=end.clone();
        let res_l = integrate(range_l.clone(), func.clone());
        let res_r = integrate(range_r.clone(), func.clone());

        let delta = res_l.clone() + res_r.clone() - last_res.clone();
        let delta = delta.sqr_abs().sqrt();
        if delta <= 15.0 * TOL || avail_depth == 0 {
            last_res
        } else {
            inner(range_l, func.clone(), res_l, avail_depth - 1)
                + inner(range_r, func.clone(), res_r, avail_depth - 1)
        }
    }

    let last_res = integrate(range.clone(), func.clone());

    inner(range, func, last_res, 16)
}

pub fn convert_to_fourier_series<T: Float + NumOps>(
    func: impl Fn(T) -> Complex<T>,
    n: usize,
) -> FourierSeriesDesc<T>
where
    Complex<T>: Mul<Complex<f64>, Output = Complex<T>> + Mul<f64, Output = Complex<T>>,
    T: Mul<f64, Output = T> + SqrAbs,
{
    assert!(n % 2 != 0);
    let half_range = ((n - 1) / 2) as isize;

    let mut coefficient_vec = Vec::new();
    for i in -half_range..=half_range {
        coefficient_vec.push(integrate_v2(T::zero()..=T::one(), |t| {
            func(t) * Complex::new(T::zero(), -t * i as f64 * 2.0 * std::f64::consts::PI).exp()
        }));
    }

    FourierSeriesDesc {
        coefficients: coefficient_vec,
    }
}
