use crate::{
    Handler, State, UseContext,
    prelude::{RouteContext, split_path},
};

mod private {
    pub trait Sealed {}
    impl Sealed for crate::Hooks<'_, '_> {}
}

pub trait UseNavigate: private::Sealed {
    fn use_navigate(&mut self) -> Handler<'static, String>;
}

impl UseNavigate for crate::Hooks<'_, '_> {
    fn use_navigate(&mut self) -> Handler<'static, String> {
        let mut route_context = self.use_context_mut::<State<RouteContext>>();
        let route_context = route_context.clone();

        Handler::from(move |path: String| {
            route_context.write().path = split_path(&path);
        })
    }
}
