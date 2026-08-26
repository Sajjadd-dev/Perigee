/** @type {import('next').NextConfig} */
const createNextIntlPlugin = require('next-intl/plugin');
const withNextIntl = createNextIntlPlugin('./lib/i18n.ts');

const nextConfig = {
  reactStrictMode: true,
  transpilePackages: ["@creit.tech/stellar-wallets-kit"],
  turbopack: {},

  images: {
    remotePatterns: [
      {
        protocol: "https",
        hostname: "stellar.creit.tech",
      },
    ],
  },

  // MIGRATION STATUS: Pages Router active
  // Explicit configuration to maintain Pages Router stability during migration.
  // Both pages/ and app/ directories can coexist during incremental migration.
  // Remove this comment and any Pages Router-specific config after full migration to App Router.
  // See MIGRATION.md for incremental migration plan.
  useFileSystemPublicRoutes: true,

  // Pin to current stable behaviour
  // Remove experimental flag when migrating to App Router
  experimental: {
    // App Router will be enabled incrementally
    // See MIGRATION.md for migration phases
  },

  async headers() {
    return [
      {
        source: "/:path*.wasm",
        headers: [
          {
            key: "Cache-Control",
            value: "public, max-age=31536000, immutable",
          },
        ],
      },
      {
        source: "/api/wasm/:path*",
        headers: [
          {
            key: "Cache-Control",
            value: "no-cache, no-store, must-revalidate",
          },
        ],
      },
    ];
  },
};

module.exports = withNextIntl(nextConfig);
