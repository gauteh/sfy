const { createProxyMiddleware } = require('http-proxy-middleware');

const target = process.env.SFY_SERVER;

module.exports = function (app) {
  if (!target) {
    console.warn('[proxy] SFY_SERVER is not set — API requests will not be proxied.');
    return;
  }

  console.log(`[proxy] Proxying /buoys/ -> ${target}`);

  app.use(
    '/buoys/',
    createProxyMiddleware({
      target,
      changeOrigin: true,
    })
  );
};
