pub trait ServiceFactory<K> {
    type Service;

    fn get_service(&self, key: K) -> Self::Service;
}

// Allows creation of services with service_fn
impl<F, T, S> ServiceFactory<T> for F
where
    F: Fn(T) -> S,
{
    type Service = S;

    fn get_service(&self, target: T) -> Self::Service {
        (self)(target)
    }
}
