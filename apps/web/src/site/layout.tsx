import { Link } from "@tanstack/react-router";
import { Menu } from "lucide-react";
import { useState, type ReactNode } from "react";
import { Button } from "../components/ui/pop-button";
import {
  Sheet,
  SheetContent,
  SheetTitle,
  SheetDescription,
  SheetTrigger,
} from "../components/ui/pop-sheet";
import { termsUrl, privacyUrl } from "../lib/legal";
import { repository } from "./content";
export function Brand() {
  return (
    <a href="/" className="brand">
      <img src="/icon.png" width="36" height="36" alt="" />
      <span>
        <strong>
          CatDo
          <span className="brand-spark" aria-hidden="true">
            ✦
          </span>
        </strong>
        <small>by WorkerCat</small>
      </span>
    </a>
  );
}
const links = [
  ["/#features", "Features"],
  ["/#download", "Download"],
  ["/#help", "Help"],
] as const;
export function SiteLayout({ children }: { children: ReactNode }) {
  const [menu, setMenu] = useState(false);
  const navigation = links.map(([to, label]) => (
    <a key={to} href={to} onClick={() => setMenu(false)}>
      {label}
    </a>
  ));
  return (
    <div className="site">
      <a className="skip-link" href="#main">
        Skip to content
      </a>
      <header className="site-header">
        <Brand />
        <nav aria-label="Main navigation" className="site-nav">
          {navigation}
        </nav>
        <Button
          variant="outline"
          size="sm"
          className="site-open"
          nativeButton={false}
          render={<a href="/app" />}
        >
          Open CatDo <span aria-hidden="true">↗</span>
        </Button>
        <Sheet open={menu} onOpenChange={setMenu}>
          <SheetTrigger
            render={
              <Button
                className="site-menu"
                variant="ghost"
                size="icon"
                aria-label="Open menu"
              />
            }
          >
            <Menu />
          </SheetTrigger>
          <SheetContent side="right" className="nav-dialog">
            <SheetTitle>Explore CatDo</SheetTitle>
            <SheetDescription className="sr-only">
              Sections on the homepage
            </SheetDescription>
            <nav>{navigation}</nav>
          </SheetContent>
        </Sheet>
      </header>
      {children}
      <footer className="site-footer">
        <div>
          <Brand />
          <p>Small tools. Room to think.</p>
        </div>
        <nav aria-label="Footer">
          <a href="https://workercat.com">WorkerCat ↗</a>
          <a href={repository}>Source code ↗</a>
          <a href={`${repository}/releases`}>Release notes ↗</a>
          <Link to="/support">Support</Link>
          <Link to="/privacy">Privacy &amp; your data</Link>
          <a href={privacyUrl}>Privacy policy</a>
          <a href={termsUrl}>Terms</a>
          <a href={`${repository}/security/policy`}>Security</a>
          <a href={`${repository}/blob/main/LICENSE.md`}>License</a>
        </nav>
        <p className="license-note">
          The hosted service is free for personal and workplace use. Source code
          has a separate PolyForm Noncommercial 1.0.0 license.
        </p>
      </footer>
    </div>
  );
}
export function PageIntro({
  eyebrow,
  title,
  children,
}: {
  eyebrow: string;
  title: string;
  children: ReactNode;
}) {
  return (
    <header className="page-intro">
      <p className="eyebrow">{eyebrow}</p>
      <h1>{title}</h1>
      <p>{children}</p>
    </header>
  );
}
