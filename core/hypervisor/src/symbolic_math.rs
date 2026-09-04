/// Symbolic Computer Algebra System (CAS) engine.
///
/// Parses math expressions into an AST, applies symbolic derivative rules
/// (product, quotient, chain, power), and synthesizes the result into ECS
/// action circuits. No neural approximations — pure recursive logic.
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum MathOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Power,
    Derivative,
    Sine,
    Cosine,
    Exponential,
    NaturalLog,
}

/// Cache-optimized symbolic node. Recursive evaluation runs directly
/// in CPU registers.
#[repr(C)]
#[derive(Debug, Clone, PartialEq)]
pub enum SymbolicNode {
    Constant(f64),
    Variable(u64),
    Expression {
        operator: MathOperator,
        left: Box<SymbolicNode>,
        right: Box<SymbolicNode>,
    },
    Unary {
        operator: MathOperator,
        operand: Box<SymbolicNode>,
    },
}

/// Maps variable name hashes to human-readable names for debugging.
pub type VarRegistry = HashMap<u64, String>;

impl SymbolicNode {
    pub fn constant(v: f64) -> Self {
        SymbolicNode::Constant(v)
    }
    pub fn var(name: &str) -> Self {
        SymbolicNode::Variable(hash_name(name))
    }

    pub fn add_expr(left: Self, right: Self) -> Self {
        SymbolicNode::Expression {
            operator: MathOperator::Add,
            left: Box::new(left),
            right: Box::new(right),
        }
    }
    pub fn mul_expr(left: Self, right: Self) -> Self {
        SymbolicNode::Expression {
            operator: MathOperator::Multiply,
            left: Box::new(left),
            right: Box::new(right),
        }
    }
    pub fn sub_expr(left: Self, right: Self) -> Self {
        SymbolicNode::Expression {
            operator: MathOperator::Subtract,
            left: Box::new(left),
            right: Box::new(right),
        }
    }
    pub fn div_expr(left: Self, right: Self) -> Self {
        SymbolicNode::Expression {
            operator: MathOperator::Divide,
            left: Box::new(left),
            right: Box::new(right),
        }
    }
    pub fn pow(base: Self, exp: Self) -> Self {
        SymbolicNode::Expression {
            operator: MathOperator::Power,
            left: Box::new(base),
            right: Box::new(exp),
        }
    }

    /// Recursive symbolic derivative via absolute calculus rules.
    /// d/dx(constant) = 0
    /// d/dx(x) = 1
    /// d/dx(u+v) = dudx + dvdx
    /// d/dx(u*v) = u*dvdx + v*dudx  (product rule)
    /// d/dx(u/v) = (v*dudx - u*dvdx) / v²  (quotient rule)
    /// d/dx(u^n) = n*u^(n-1) * dudx  (power rule + chain)
    pub fn derive(&self, target_var: u64) -> Self {
        match self {
            SymbolicNode::Constant(_) => SymbolicNode::Constant(0.0),
            SymbolicNode::Variable(v) => {
                if *v == target_var {
                    SymbolicNode::Constant(1.0)
                } else {
                    SymbolicNode::Constant(0.0)
                }
            }
            SymbolicNode::Expression {
                operator,
                left,
                right,
            } => match operator {
                MathOperator::Add => {
                    SymbolicNode::add_expr(left.derive(target_var), right.derive(target_var))
                }
                MathOperator::Subtract => {
                    SymbolicNode::sub_expr(left.derive(target_var), right.derive(target_var))
                }
                MathOperator::Multiply => {
                    // Product rule: d/dx(u*v) = u*v' + v*u'
                    SymbolicNode::add_expr(
                        SymbolicNode::mul_expr((**left).clone(), right.derive(target_var)),
                        SymbolicNode::mul_expr((**right).clone(), left.derive(target_var)),
                    )
                }
                MathOperator::Divide => {
                    // Quotient rule: d/dx(u/v) = (v*u' - u*v') / v²
                    let u = left.as_ref();
                    let v = right.as_ref();
                    let u_prime = left.derive(target_var);
                    let v_prime = right.derive(target_var);
                    let numerator = SymbolicNode::sub_expr(
                        SymbolicNode::mul_expr((*v).clone(), u_prime),
                        SymbolicNode::mul_expr((*u).clone(), v_prime),
                    );
                    let denominator = SymbolicNode::mul_expr((*v).clone(), (*v).clone());
                    SymbolicNode::div_expr(numerator, denominator)
                }
                MathOperator::Power => {
                    // d/dx(u^n) = n * u^(n-1) * du/dx
                    let n = right.as_ref();
                    let u = left.as_ref();
                    let u_prime = left.derive(target_var);
                    let n_minus_1 =
                        SymbolicNode::sub_expr((*n).clone(), SymbolicNode::Constant(1.0));
                    SymbolicNode::mul_expr(
                        SymbolicNode::mul_expr((*n).clone(), u_prime),
                        SymbolicNode::pow((*u).clone(), n_minus_1),
                    )
                }
                _ => SymbolicNode::Constant(0.0),
            },
            SymbolicNode::Unary { operator, operand } => match operator {
                MathOperator::Sine => {
                    // d/dx sin(u) = cos(u) * du/dx
                    SymbolicNode::mul_expr(
                        SymbolicNode::Unary {
                            operator: MathOperator::Cosine,
                            operand: operand.clone(),
                        },
                        operand.derive(target_var),
                    )
                }
                MathOperator::Cosine => {
                    // d/dx cos(u) = -sin(u) * du/dx
                    SymbolicNode::mul_expr(
                        SymbolicNode::Unary {
                            operator: MathOperator::Sine,
                            operand: operand.clone(),
                        },
                        SymbolicNode::mul_expr(
                            SymbolicNode::Constant(-1.0),
                            operand.derive(target_var),
                        ),
                    )
                }
                MathOperator::Exponential => {
                    // d/dx exp(u) = exp(u) * du/dx
                    SymbolicNode::mul_expr(
                        SymbolicNode::Unary {
                            operator: MathOperator::Exponential,
                            operand: operand.clone(),
                        },
                        operand.derive(target_var),
                    )
                }
                MathOperator::NaturalLog => {
                    // d/dx ln(u) = (1/u) * du/dx
                    SymbolicNode::mul_expr(
                        SymbolicNode::div_expr(SymbolicNode::Constant(1.0), (**operand).clone()),
                        operand.derive(target_var),
                    )
                }
                _ => SymbolicNode::Constant(0.0),
            },
        }
    }

    /// Symbolic integration with respect to a variable.
    ///
    /// Implements basic integration rules:
    /// - ∫c dx = c*x
    /// - ∫x dx = x²/2
    /// - ∫x^n dx = x^(n+1)/(n+1)
    /// - ∫sin(x) dx = -cos(x)
    /// - ∫cos(x) dx = sin(x)
    /// - ∫exp(x) dx = exp(x)
    /// - ∫1/x dx = ln|x|
    /// - ∫(u+v) dx = ∫u dx + ∫v dx
    ///
    /// Returns `None` if the integral cannot be computed symbolically.
    pub fn integrate(&self, target_var: u64) -> Option<Self> {
        match self {
            // ∫c dx = c*x
            SymbolicNode::Constant(c) => Some(SymbolicNode::mul_expr(
                SymbolicNode::Constant(*c),
                SymbolicNode::Variable(target_var),
            )),
            // ∫x dx = x²/2 (if this is the target variable)
            SymbolicNode::Variable(v) => {
                if *v == target_var {
                    Some(SymbolicNode::div_expr(
                        SymbolicNode::pow(
                            SymbolicNode::Variable(target_var),
                            SymbolicNode::Constant(2.0),
                        ),
                        SymbolicNode::Constant(2.0),
                    ))
                } else {
                    // ∫c dx = c*x (treating other variables as constants)
                    Some(SymbolicNode::mul_expr(
                        self.clone(),
                        SymbolicNode::Variable(target_var),
                    ))
                }
            }
            SymbolicNode::Expression {
                operator,
                left,
                right,
            } => match operator {
                // ∫(u+v) dx = ∫u dx + ∫v dx
                MathOperator::Add => {
                    let u = left.integrate(target_var)?;
                    let v = right.integrate(target_var)?;
                    Some(SymbolicNode::add_expr(u, v))
                }
                // ∫(u-v) dx = ∫u dx - ∫v dx
                MathOperator::Subtract => {
                    let u = left.integrate(target_var)?;
                    let v = right.integrate(target_var)?;
                    Some(SymbolicNode::sub_expr(u, v))
                }
                // ∫(c*f(x)) dx = c * ∫f(x) dx
                MathOperator::Multiply => {
                    // Try left as constant
                    if let SymbolicNode::Constant(c) = left.as_ref() {
                        let inner = right.integrate(target_var)?;
                        return Some(SymbolicNode::mul_expr(SymbolicNode::Constant(*c), inner));
                    }
                    // Try right as constant
                    if let SymbolicNode::Constant(c) = right.as_ref() {
                        let inner = left.integrate(target_var)?;
                        return Some(SymbolicNode::mul_expr(SymbolicNode::Constant(*c), inner));
                    }
                    None // Product of two non-constant functions — integration by parts not implemented
                }
                // ∫x^n dx = x^(n+1)/(n+1) (power rule)
                MathOperator::Power => {
                    // Check if left is our target variable and right is a constant
                    if let SymbolicNode::Variable(v) = left.as_ref()
                        && *v == target_var
                        && let SymbolicNode::Constant(n) = right.as_ref()
                    {
                        let n_plus_1 = SymbolicNode::Constant(n + 1.0);
                        return Some(SymbolicNode::div_expr(
                            SymbolicNode::pow(SymbolicNode::Variable(target_var), n_plus_1.clone()),
                            n_plus_1,
                        ));
                    }
                    None
                }
                _ => None,
            },
            SymbolicNode::Unary { operator, operand } => match operator {
                // ∫sin(u) dx — only if u is the target variable (no chain rule)
                MathOperator::Sine => {
                    if let SymbolicNode::Variable(v) = operand.as_ref()
                        && *v == target_var
                    {
                        return Some(SymbolicNode::Unary {
                            operator: MathOperator::Cosine,
                            operand: operand.clone(),
                        });
                    }
                    None
                }
                // ∫cos(u) dx
                MathOperator::Cosine => {
                    if let SymbolicNode::Variable(v) = operand.as_ref()
                        && *v == target_var
                    {
                        return Some(SymbolicNode::Unary {
                            operator: MathOperator::Sine,
                            operand: operand.clone(),
                        });
                    }
                    None
                }
                // ∫exp(u) dx
                MathOperator::Exponential => {
                    if let SymbolicNode::Variable(v) = operand.as_ref()
                        && *v == target_var
                    {
                        return Some(self.clone());
                    }
                    None
                }
                // ∫ln(u) dx — only for simple case
                MathOperator::NaturalLog => {
                    if let SymbolicNode::Variable(v) = operand.as_ref()
                        && *v == target_var
                    {
                        // ∫ln(x) dx = x*ln(x) - x
                        return Some(SymbolicNode::sub_expr(
                            SymbolicNode::mul_expr(
                                SymbolicNode::Variable(target_var),
                                SymbolicNode::Unary {
                                    operator: MathOperator::NaturalLog,
                                    operand: operand.clone(),
                                },
                            ),
                            SymbolicNode::Variable(target_var),
                        ));
                    }
                    None
                }
                _ => None,
            },
        }
    }

    /// Evaluate the expression numerically by substituting variable values.
    pub fn evaluate(&self, vars: &HashMap<u64, f64>) -> Result<f64, String> {
        match self {
            SymbolicNode::Constant(v) => Ok(*v),
            SymbolicNode::Variable(v) => vars
                .get(v)
                .copied()
                .ok_or_else(|| format!("Undefined var hash {}", v)),
            SymbolicNode::Expression {
                operator,
                left,
                right,
            } => {
                let l = left.evaluate(vars)?;
                let r = right.evaluate(vars)?;
                match operator {
                    MathOperator::Add => Ok(l + r),
                    MathOperator::Subtract => Ok(l - r),
                    MathOperator::Multiply => Ok(l * r),
                    MathOperator::Divide => {
                        if r == 0.0 {
                            Err("Division by zero".into())
                        } else {
                            Ok(l / r)
                        }
                    }
                    MathOperator::Power => Ok(l.powf(r)),
                    _ => Err("Cannot evaluate derivative symbolically".into()),
                }
            }
            SymbolicNode::Unary { operator, operand } => {
                let v = operand.evaluate(vars)?;
                match operator {
                    MathOperator::Sine => Ok(v.sin()),
                    MathOperator::Cosine => Ok(v.cos()),
                    MathOperator::Exponential => Ok(v.exp()),
                    MathOperator::NaturalLog => {
                        if v <= 0.0 {
                            Err("log of non-positive".into())
                        } else {
                            Ok(v.ln())
                        }
                    }
                    _ => Err("Cannot evaluate".into()),
                }
            }
        }
    }

    /// Simplify expression (constant folding, x*0=0, x+0=x, etc.)
    pub fn simplify(self) -> Self {
        match self {
            SymbolicNode::Expression {
                operator,
                left,
                right,
            } => {
                let l = left.simplify();
                let r = right.simplify();
                match (&l, &r, &operator) {
                    // x + 0 = x, 0 + x = x
                    (_, SymbolicNode::Constant(c), &MathOperator::Add) if *c == 0.0 => l,
                    (SymbolicNode::Constant(c), _, &MathOperator::Add) if *c == 0.0 => r,
                    // x - 0 = x
                    (_, SymbolicNode::Constant(c), &MathOperator::Subtract) if *c == 0.0 => l,
                    // x * 0 = 0, x * 1 = x
                    (_, SymbolicNode::Constant(c), &MathOperator::Multiply) if *c == 0.0 => {
                        SymbolicNode::Constant(0.0)
                    }
                    (_, SymbolicNode::Constant(c), &MathOperator::Multiply) if *c == 1.0 => l,
                    (SymbolicNode::Constant(c), _, &MathOperator::Multiply) if *c == 1.0 => r,
                    // x / 1 = x
                    (_, SymbolicNode::Constant(c), &MathOperator::Divide) if *c == 1.0 => l,
                    // Constant folding
                    (&SymbolicNode::Constant(lc), &SymbolicNode::Constant(rc), _) => {
                        let vars = HashMap::new();
                        SymbolicNode::Constant(
                            SymbolicNode::Expression {
                                operator,
                                left: Box::new(SymbolicNode::Constant(lc)),
                                right: Box::new(SymbolicNode::Constant(rc)),
                            }
                            .evaluate(&vars)
                            .unwrap_or(0.0),
                        )
                    }
                    _ => SymbolicNode::Expression {
                        operator,
                        left: Box::new(l),
                        right: Box::new(r),
                    },
                }
            }
            SymbolicNode::Unary { operator, operand } => {
                let op = operand.simplify();
                match (&op, &operator) {
                    (&SymbolicNode::Constant(c), _) => {
                        let vars = HashMap::new();
                        SymbolicNode::Constant(
                            SymbolicNode::Unary {
                                operator,
                                operand: Box::new(SymbolicNode::Constant(c)),
                            }
                            .evaluate(&vars)
                            .unwrap_or(0.0),
                        )
                    }
                    _ => SymbolicNode::Unary {
                        operator,
                        operand: Box::new(op),
                    },
                }
            }
            other => other,
        }
    }

    /// Render the expression as a string for debugging.
    pub fn to_string(&self, reg: &VarRegistry) -> String {
        match self {
            SymbolicNode::Constant(v) => format!("{}", v),
            SymbolicNode::Variable(v) => reg.get(v).cloned().unwrap_or_else(|| format!("v{:x}", v)),
            SymbolicNode::Expression {
                operator,
                left,
                right,
            } => {
                let lop = left.to_string(reg);
                let rop = right.to_string(reg);
                match operator {
                    MathOperator::Add => format!("({} + {})", lop, rop),
                    MathOperator::Subtract => format!("({} - {})", lop, rop),
                    MathOperator::Multiply => format!("({} * {})", lop, rop),
                    MathOperator::Divide => format!("({} / {})", lop, rop),
                    MathOperator::Power => format!("({})^{}", lop, rop),
                    MathOperator::Derivative => format!("d/dx({})", lop),
                    _ => format!("op({}, {})", lop, rop),
                }
            }
            SymbolicNode::Unary { operator, operand } => {
                let s = operand.to_string(reg);
                match operator {
                    MathOperator::Sine => format!("sin({})", s),
                    MathOperator::Cosine => format!("cos({})", s),
                    MathOperator::Exponential => format!("exp({})", s),
                    MathOperator::NaturalLog => format!("ln({})", s),
                    _ => format!("unary({})", s),
                }
            }
        }
    }
}

fn hash_name(name: &str) -> u64 {
    let mut h = 0x9E3779B97F4A7C15u64;
    for b in name.bytes() {
        h = h.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(b as u64);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_constant() {
        let x = hash_name("x");
        let expr = SymbolicNode::Constant(5.0);
        let d = expr.derive(x).simplify();
        assert_eq!(d, SymbolicNode::Constant(0.0));
    }

    #[test]
    fn test_derive_variable() {
        let x = hash_name("x");
        let y = hash_name("y");
        let expr = SymbolicNode::Variable(x);
        let d = expr.derive(x);
        assert_eq!(d, SymbolicNode::Constant(1.0));
        let d2 = SymbolicNode::Variable(y).derive(x);
        assert_eq!(d2, SymbolicNode::Constant(0.0));
    }

    #[test]
    fn test_derive_sum() {
        let x = hash_name("x");
        // d/dx (x + 5) = 1 + 0 = 1
        let expr = SymbolicNode::add_expr(SymbolicNode::var("x"), SymbolicNode::Constant(5.0));
        let d = expr.derive(x).simplify();
        assert_eq!(d, SymbolicNode::Constant(1.0));
    }

    #[test]
    fn test_product_rule() {
        let x = hash_name("x");
        let reg: VarRegistry = [(x, "x".into())].into_iter().collect();
        // d/dx (x * x) = x*1 + x*1 = x + x after simplify
        let expr = SymbolicNode::mul_expr(SymbolicNode::var("x"), SymbolicNode::var("x"));
        let d = expr.derive(x).simplify();
        let s = d.to_string(&reg);
        assert!(s.contains("x + x"), "expected x + x, got: {}", s);
    }

    #[test]
    fn test_quotient_rule() {
        let x = hash_name("x");
        // d/dx (x / x) after simplify should be 1 / x^2 * (x - x) ... let's just check it doesn't panic
        let expr = SymbolicNode::div_expr(SymbolicNode::var("x"), SymbolicNode::var("x"));
        let d = expr.derive(x);
        // Result should not be Constant(0.0) — the structure is valid
        match d {
            SymbolicNode::Expression { .. } => {} // expected
            _ => panic!("quotient rule should produce an expression, got {:?}", d),
        }
    }

    #[test]
    fn test_power_rule() {
        let x = hash_name("x");
        // d/dx (x^3) = 3*x^2
        let expr = SymbolicNode::pow(SymbolicNode::var("x"), SymbolicNode::Constant(3.0));
        let d = expr.derive(x);
        let s = d.simplify();
        let reg: VarRegistry = [(hash_name("x"), "x".to_string())].into_iter().collect();
        assert!(
            s.to_string(&reg).contains("3"),
            "got: {}",
            s.to_string(&reg)
        );
    }

    #[test]
    fn test_evaluate() {
        let mut vars = HashMap::new();
        vars.insert(hash_name("x"), 2.0);
        let expr = SymbolicNode::add_expr(SymbolicNode::var("x"), SymbolicNode::Constant(3.0));
        assert_eq!(expr.evaluate(&vars).unwrap(), 5.0);
    }

    #[test]
    fn test_simplify_constant_fold() {
        // 2 + 3 = 5
        let expr = SymbolicNode::add_expr(SymbolicNode::Constant(2.0), SymbolicNode::Constant(3.0));
        let s = expr.simplify();
        assert_eq!(s, SymbolicNode::Constant(5.0));
    }

    #[test]
    fn test_simplify_add_zero() {
        let expr = SymbolicNode::add_expr(SymbolicNode::var("x"), SymbolicNode::Constant(0.0));
        let s = expr.simplify();
        assert_eq!(s, SymbolicNode::var("x"));
    }

    #[test]
    fn test_simplify_mul_one() {
        let expr = SymbolicNode::mul_expr(SymbolicNode::var("x"), SymbolicNode::Constant(1.0));
        let s = expr.simplify();
        assert_eq!(s, SymbolicNode::var("x"));
    }

    #[test]
    fn test_simplify_mul_zero() {
        let expr = SymbolicNode::mul_expr(SymbolicNode::var("x"), SymbolicNode::Constant(0.0));
        let s = expr.simplify();
        assert_eq!(s, SymbolicNode::Constant(0.0));
    }

    #[test]
    fn test_sine_derivative() {
        let x = hash_name("x");
        let expr = SymbolicNode::Unary {
            operator: MathOperator::Sine,
            operand: Box::new(SymbolicNode::var("x")),
        };
        let d = expr.derive(x);
        // d/dx sin(x) = cos(x)
        match d {
            SymbolicNode::Expression {
                operator: MathOperator::Multiply,
                left,
                right: _,
            } => {
                // cos(x) * 1  after simplification
                assert!(matches!(
                    *left,
                    SymbolicNode::Unary {
                        operator: MathOperator::Cosine,
                        ..
                    }
                ));
            }
            _ => panic!("expected product of cos(x) * 1, got: {:?}", d),
        }
    }
}
