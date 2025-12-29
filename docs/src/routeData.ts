import { defineRouteMiddleware } from "@astrojs/starlight/route-data";

export const onRequest = defineRouteMiddleware((context) => {
    const { entry } = context.locals.starlightRoute;
    if (entry.id.startsWith('principle')) {
        context.locals.starlightRoute.sidebar=context.locals.starlightRoute.sidebar.splice(2,1)
    } else {
        context.locals.starlightRoute.sidebar.splice(2,1)
    }
})