//! Category Theory abstractions for compositional guarantees.
//! Provides functors, natural transformations, and adjunctions to ensure
//! structure preservation across domain transformations.

/// Type aliases for complex callback signatures.
type ConstraintFn<X, Y, Z> = Box<dyn Fn(&X, &Y) -> Z>;
type IdentificationFn<Z, X, Y> = Box<dyn Fn(&Z) -> (X, Y)>;
type EquivalenceFn = Box<dyn Fn(&[f64], &[f64]) -> bool>;
type InterfaceMapFn = Box<dyn Fn(&[String]) -> Vec<String>>;
type BehaviorMapFn = Box<dyn Fn(&[f64]) -> Vec<f64>>;

/// Trait for types that form a Category.
/// Must satisfy:
/// 1. Associativity: (f ∘ g) ∘ h = f ∘ (g ∘ h)
/// 2. Identity: id ∘ f = f ∘ id = f
pub trait Category {
    type Object;
    fn id() -> Self;
    fn compose(self, other: Self) -> Self;
}

/// Trait for Functors.
/// Maps between categories while preserving structure.
/// Must satisfy:
/// 1. F(id) = id
/// 2. F(f ∘ g) = F(f) ∘ F(g)
pub trait Functor {
    type Input;
    type Output;

    /// fmap: apply function inside the functor context
    fn fmap<F: Fn(Self::Input) -> Self::Output>(self, f: F) -> Self;
}

/// Trait for Natural Transformations.
/// Maps between functors while preserving composition.
/// η: F → G such that η_Y ∘ F(f) = G(f) ∘ η_X
pub trait NaturalTransformation<F, G>
where
    F: Functor,
    G: Functor,
{
    fn apply(&self, x: &F::Input) -> G::Output;
}

/// Trait for Monads.
/// Represents computations with context.
/// Must satisfy monad laws:
/// 1. return x >>= f = f x
/// 2. m >>= return = m
/// 3. (m >>= f) >>= g = m >>= (\x -> f x >>= g)
pub trait Monad: Functor {
    fn unit(x: Self::Input) -> Self;
    fn bind<F>(self, f: F) -> Self
    where
        F: Fn(Self::Input) -> Self;
}

/// Option monad for computations that may fail.
#[derive(Debug, Clone, PartialEq)]
pub enum OptionMonad<T> {
    Some(T),
    None,
}

impl<T> OptionMonad<T> {
    pub fn map<U, F: Fn(T) -> U>(self, f: F) -> OptionMonad<U> {
        match self {
            OptionMonad::Some(x) => OptionMonad::Some(f(x)),
            OptionMonad::None => OptionMonad::None,
        }
    }

    pub fn and_then<U, F: Fn(T) -> OptionMonad<U>>(self, f: F) -> OptionMonad<U> {
        match self {
            OptionMonad::Some(x) => f(x),
            OptionMonad::None => OptionMonad::None,
        }
    }
}

impl<T> Functor for OptionMonad<T> {
    type Input = T;
    type Output = T;

    fn fmap<F: Fn(T) -> T>(self, f: F) -> Self {
        self.map(f)
    }
}

impl<T> Monad for OptionMonad<T> {
    fn unit(x: T) -> Self {
        OptionMonad::Some(x)
    }

    fn bind<F>(self, f: F) -> Self
    where
        F: Fn(T) -> Self,
    {
        self.and_then(f)
    }
}

/// Result monad for computations that may fail with error.
#[derive(Debug, Clone)]
pub enum ResultMonad<T, E> {
    Ok(T),
    Err(E),
}

impl<T, E> ResultMonad<T, E> {
    pub fn map<U, F: Fn(T) -> U>(self, f: F) -> ResultMonad<U, E> {
        match self {
            ResultMonad::Ok(x) => ResultMonad::Ok(f(x)),
            ResultMonad::Err(e) => ResultMonad::Err(e),
        }
    }

    pub fn and_then<U, F: Fn(T) -> ResultMonad<U, E>>(self, f: F) -> ResultMonad<U, E> {
        match self {
            ResultMonad::Ok(x) => f(x),
            ResultMonad::Err(e) => ResultMonad::Err(e),
        }
    }
}

impl<T, E> Functor for ResultMonad<T, E> {
    type Input = T;
    type Output = T;

    fn fmap<F: Fn(T) -> T>(self, f: F) -> Self {
        self.map(f)
    }
}

impl<T, E> Monad for ResultMonad<T, E> {
    fn unit(x: T) -> Self {
        ResultMonad::Ok(x)
    }

    fn bind<F>(self, f: F) -> Self
    where
        F: Fn(T) -> Self,
    {
        self.and_then(f)
    }
}

/// Adjunction: pair of functors F ⊣ G where F is left adjoint to G.
/// Hom(F(X), Y) ≅ Hom(X, G(Y))
/// Represents optimal approximation between domains.
pub struct Adjunction<F, G> {
    pub left_adjunct: F,
    pub right_adjunct: G,
}

impl<F, G> Adjunction<F, G> {
    pub fn new(left: F, right: G) -> Self {
        Self {
            left_adjunct: left,
            right_adjunct: right,
        }
    }
}

/// Limit: universal cone over a diagram.
/// Represents the "best" object that maps to all objects in the diagram.
pub trait Limit<D> {
    type Object;
    fn projection(&self, index: usize) -> Self::Object;
}

/// Colimit: universal cocone under a diagram.
/// Represents the "best" object that all objects in the diagram map to.
pub trait Colimit<D> {
    type Object;
    fn injection(&self, index: usize) -> Self::Object;
}

/// Product: categorical product (limit of discrete diagram).
/// (A × B, π₁, π₂) where π₁: A×B → A, π₂: A×B → B
#[derive(Debug, Clone)]
pub struct Product<A, B> {
    pub first: A,
    pub second: B,
}

impl<A, B> Product<A, B> {
    pub fn new(first: A, second: B) -> Self {
        Self { first, second }
    }

    pub fn fst(&self) -> &A {
        &self.first
    }

    pub fn snd(&self) -> &B {
        &self.second
    }
}

/// Coproduct: categorical sum (colimit of discrete diagram).
/// A + B with injections i₁: A → A+B, i₂: B → A+B
#[derive(Debug, Clone)]
pub enum Coproduct<A, B> {
    Left(A),
    Right(B),
}

/// Pullback: limit of cospan X → Z ← Y.
/// Represents the "best" object that maps to both X and Y consistently.
pub struct Pullback<X, Y, Z> {
    pub object: Product<X, Y>,
    pub constraint: ConstraintFn<X, Y, Z>,
}

impl<X, Y, Z> Pullback<X, Y, Z> {
    pub fn new(x: X, y: Y, constraint: impl Fn(&X, &Y) -> Z + 'static) -> Self {
        Self {
            object: Product::new(x, y),
            constraint: Box::new(constraint),
        }
    }

    pub fn satisfies_constraint(&self) -> Z {
        (self.constraint)(&self.object.first, &self.object.second)
    }
}

/// Pushout: colimit of span X ← Z → Y.
/// Represents the "best" object that both X and Y map to consistently.
pub struct Pushout<X, Y, Z> {
    pub result: Coproduct<X, Y>,
    pub identification: IdentificationFn<Z, X, Y>,
}

/// Natural transformation between component pipelines.
/// Ensures that different analysis pipelines produce equivalent results.
pub struct PipelineNaturalTransformation {
    pub name: String,
    pub source_pipeline: String,
    pub target_pipeline: String,
    pub equivalence_test: EquivalenceFn,
}

impl PipelineNaturalTransformation {
    pub fn new(
        name: &str,
        source: &str,
        target: &str,
        test: impl Fn(&[f64], &[f64]) -> bool + 'static,
    ) -> Self {
        Self {
            name: name.to_string(),
            source_pipeline: source.to_string(),
            target_pipeline: target.to_string(),
            equivalence_test: Box::new(test),
        }
    }

    pub fn verify(&self, source_output: &[f64], target_output: &[f64]) -> bool {
        (self.equivalence_test)(source_output, target_output)
    }
}

/// Functor for component composition.
/// Maps component interfaces while preserving behavioral contracts.
pub struct ComponentFunctor {
    pub name: String,
    pub map_interface: InterfaceMapFn,
    pub map_behavior: BehaviorMapFn,
}

impl ComponentFunctor {
    pub fn new(
        name: &str,
        interface_map: impl Fn(&[String]) -> Vec<String> + 'static,
        behavior_map: impl Fn(&[f64]) -> Vec<f64> + 'static,
    ) -> Self {
        Self {
            name: name.to_string(),
            map_interface: Box::new(interface_map),
            map_behavior: Box::new(behavior_map),
        }
    }

    pub fn apply_interface(&self, interface: &[String]) -> Vec<String> {
        (self.map_interface)(interface)
    }

    pub fn apply_behavior(&self, behavior: &[f64]) -> Vec<f64> {
        (self.map_behavior)(behavior)
    }
}

/// Yoneda embedding: represents objects by their relationships.
/// A ≅ Hom(-, A)
/// Used for component discovery via interface matching.
pub struct YonedaEmbedding<T> {
    pub relationships: Vec<(String, T)>,
}

impl<T> Default for YonedaEmbedding<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> YonedaEmbedding<T> {
    pub fn new() -> Self {
        Self {
            relationships: Vec::new(),
        }
    }

    pub fn add_relationship(&mut self, name: String, value: T) {
        self.relationships.push((name, value));
    }

    pub fn find_by_relationship(&self, target: &T) -> Option<&String>
    where
        T: PartialEq,
    {
        self.relationships
            .iter()
            .find(|(_, v)| v == target)
            .map(|(name, _)| name)
    }
}

/// Composition verification.
/// Verifies that component composition preserves expected properties.
pub fn verify_composition<A, B, C>(
    f: impl Fn(A) -> B,
    g: impl Fn(B) -> C,
    input: A,
    expected: C,
    tolerance: f64,
) -> bool
where
    B: PartialEq,
    C: PartialEq + Into<f64> + Clone,
    A: Clone,
{
    let result = g(f(input.clone()));
    let result_val: f64 = result.clone().into();
    let expected_val: f64 = expected.into();
    (result_val - expected_val).abs() < tolerance
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_option_monad() {
        let some = OptionMonad::Some(5);
        let mapped = some.map(|x| x * 2);
        assert_eq!(mapped, OptionMonad::Some(10));

        let none: OptionMonad<i32> = OptionMonad::None;
        let mapped_none = none.map(|x| x * 2);
        assert_eq!(mapped_none, OptionMonad::None);
    }

    #[test]
    fn test_option_monad_laws() {
        // Left identity: unit x >>= f = f x
        let x = 5;
        let f = |x| OptionMonad::Some(x * 2);
        let left = OptionMonad::unit(x).bind(f);
        let right = f(x);
        assert_eq!(left, right);

        // Right identity: m >>= unit = m
        let m = OptionMonad::Some(5);
        let result = m.clone().bind(OptionMonad::unit);
        assert_eq!(result, m.clone());
    }

    #[test]
    fn test_product() {
        let p = Product::new(1, "hello");
        assert_eq!(*p.fst(), 1);
        assert_eq!(*p.snd(), "hello");
    }

    #[test]
    fn test_coproduct() {
        let left: Coproduct<i32, &str> = Coproduct::Left(42);
        let right: Coproduct<i32, &str> = Coproduct::Right("world");

        match left {
            Coproduct::Left(x) => assert_eq!(x, 42),
            Coproduct::Right(_) => panic!("Expected Left"),
        }

        match right {
            Coproduct::Left(_) => panic!("Expected Right"),
            Coproduct::Right(x) => assert_eq!(x, "world"),
        }
    }

    #[test]
    fn test_yoneda_embedding() {
        let mut embedding = YonedaEmbedding::new();
        embedding.add_relationship("component_a".to_string(), 0.5);
        embedding.add_relationship("component_b".to_string(), 0.8);

        assert_eq!(
            embedding.find_by_relationship(&0.5),
            Some(&"component_a".to_string())
        );
        assert_eq!(
            embedding.find_by_relationship(&0.8),
            Some(&"component_b".to_string())
        );
    }

    #[test]
    fn test_pipeline_natural_transformation() {
        let transform = PipelineNaturalTransformation::new(
            "test_transform",
            "pipeline_a",
            "pipeline_b",
            |a, b| {
                if a.len() != b.len() {
                    return false;
                }
                a.iter().zip(b.iter()).all(|(x, y)| (x - y).abs() < 0.011)
            },
        );

        let output_a = vec![0.5, 0.6, 0.7];
        let output_b = vec![0.51, 0.59, 0.71];
        assert!(transform.verify(&output_a, &output_b));
    }
}
