import { Links, Meta, Outlet, Scripts, ScrollRestoration } from "@remix-run/react";

export function Layout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <head>
        <meta charSet="utf-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1" />
        <Meta />
        <Links />
        <style>{`
          * { box-sizing: border-box; margin: 0; padding: 0; }
          body { font-family: system-ui, sans-serif; background: #f0f2f5; color: #1a1a1a; }
          a { color: #0066cc; text-decoration: none; }
          a:hover { text-decoration: underline; }
          button { cursor: pointer; }
          .container { max-width: 1200px; margin: 0 auto; padding: 2rem; }
          .card { background: white; border-radius: 8px; padding: 1.5rem; box-shadow: 0 1px 3px rgba(0,0,0,.1); }
          .btn { padding: .5rem 1rem; border: none; border-radius: 4px; font-size: .9rem; }
          .btn-primary { background: #0066cc; color: white; }
          .btn-primary:hover { background: #0052a3; }
          .btn-danger { background: #dc3545; color: white; }
          .btn-danger:hover { background: #c82333; }
          .btn-secondary { background: #6c757d; color: white; }
          input, textarea { width: 100%; padding: .5rem; border: 1px solid #ddd; border-radius: 4px; font-size: .9rem; }
          textarea { min-height: 80px; resize: vertical; }
          h1 { font-size: 1.8rem; margin-bottom: 1rem; }
          h2 { font-size: 1.4rem; margin-bottom: .75rem; }
          .error { color: red; margin: .5rem 0; }
        `}</style>
      </head>
      <body>
        {children}
        <ScrollRestoration />
        <Scripts />
      </body>
    </html>
  );
}

export default function App() {
  return <Outlet />;
}
