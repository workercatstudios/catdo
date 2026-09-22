import { useState } from "react";
import {
  Dialog,
  DialogContent,
  DialogTitle,
  DialogDescription,
} from "./ui/dialog";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
export type Naming = {
  kind: "workspace" | "project";
  id?: string;
  name: string;
};
export function NamingDialog({
  target,
  close,
  save,
}: {
  target: Naming;
  close: () => void;
  save: (name: string) => Promise<void>;
}) {
  const [name, setName] = useState(target.name),
    [error, setError] = useState(""),
    [saving, setSaving] = useState(false);
  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open && !saving) close();
      }}
    >
      <DialogContent>
        <DialogTitle>
          {target.id ? "Rename" : "New"} {target.kind}
        </DialogTitle>
        <DialogDescription>
          A clear name makes it easier to find your way back.
        </DialogDescription>
        <form
          onSubmit={async (e) => {
            e.preventDefault();
            setSaving(true);
            try {
              await save(name);
              close();
            } catch (e) {
              setError(e instanceof Error ? e.message : "Could not save.");
            } finally {
              setSaving(false);
            }
          }}
        >
          <label>
            Name
            <Input
              required
              maxLength={80}
              autoFocus
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
          </label>
          {error && (
            <p className="error" role="alert">
              {error}
            </p>
          )}
          <div className="dialog-actions">
            <Button
              variant="outline"
              type="button"
              onClick={close}
              disabled={saving}
            >
              Cancel
            </Button>
            <Button disabled={saving || !name.trim()}>
              {saving ? "Saving…" : "Save"}
            </Button>
          </div>
        </form>
      </DialogContent>
    </Dialog>
  );
}
