export const repository = "https://github.com/workercatstudios/catdo";
export const release = {
  version: "0.2.0",
  date: "2026-09-22",
  url: `${repository}/releases/tag/v0.2.0`,
  appImage: `${repository}/releases/download/v0.2.0/catdo-0.2.0-linux-x86_64.AppImage`,
  archive: `${repository}/releases/download/v0.2.0/catdo-0.2.0-linux-x86_64.tar.gz`,
};
export const guides = [
  {
    slug: "getting-started",
    title: "Your first little plan",
    description: "Start with an inbox, then give your tasks a home.",
    sections: [
      [
        "Start with what’s on your mind",
        "Open CatDo and sign in with your WorkerCat account. Your account works across the web and Linux app. Add a task in Inbox: a title is enough. You can add notes, dates, and subtasks whenever you need them.",
      ],
      [
        "Make room for different parts of life",
        "Use the workspace selector to switch between spaces. Create a workspace for personal tasks and another for work. Within each, projects keep related tasks together. Search looks inside the current workspace.",
      ],
      [
        "Give today a little shape",
        "Open a task and choose a scheduled date for when you want to work on it. Use a due date only when there’s a deadline. Today includes tasks scheduled or due today, as well as unfinished tasks from earlier days.",
      ],
      [
        "Take one small step",
        "Check a task off when you’re done. Use Undo after a change if you need it back, or reopen a task from Completed. Recurring tasks advance to their next date when completed.",
      ],
    ],
  },
  {
    slug: "dates-and-repeats",
    title: "A day to do it. A date it’s due.",
    description: "Plan with scheduled dates, deadlines, and repeating tasks.",
    sections: [
      [
        "Scheduled and due are different",
        "Scheduled is the day you plan to work on a task. Due is its deadline. A task can have either date or both. Calendar shows both, with a diamond marking a deadline. Drag a scheduled task to another day, or edit its date in Task details.",
      ],
      [
        "Repeat on a schedule",
        "Choose daily, weekly, monthly, or selected weekdays. Use the interval to repeat every two weeks, for example. Set a scheduled date or due date so CatDo knows where the series begins. Completing an occurrence advances the task and records the completed one in history.",
      ],
      [
        "Repeat after you finish",
        "Choose days, weeks, or months after completion for tasks whose next date depends on when you finish. This works well for changing a filter or checking in on a project.",
      ],
      [
        "Keep a reminder nearby",
        "Set a reminder in Task details. Notifications are delivered by the Linux app while it’s running, including in the tray. The web app can edit reminder times, but does not send background notifications.",
      ],
    ],
  },
  {
    slug: "sync-and-offline",
    title: "Keep going. Catch up later.",
    description: "How saved tasks, sync, and offline use work.",
    sections: [
      [
        "Sign in once while connected",
        "After you sign in and open your tasks online, CatDo saves them on this device. The web app stores task data in your browser; Linux uses a local database. Keep using the same browser profile to access those saved tasks.",
      ],
      [
        "Open saved tasks offline",
        "If the network or sign-in service is unavailable, choose Open saved tasks. Add, edit, and complete tasks as usual. When using this offline entry, reconnect and sign in again to resume sync. Saved tasks remain on shared devices, so use a private device or clear site data after signing out.",
      ],
      [
        "Let your devices catch up",
        "While CatDo is open, it checks for changes regularly. Sync now requests a check immediately. Use the same WorkerCat account on each device. Changes are saved locally before being sent. Closing the browser does not continue syncing in the background.",
      ],
      [
        "Resolve changes made in two places",
        "Independent edits can merge automatically. If the same information changed on two devices, CatDo asks which version to keep. Read the conflict message before choosing; some project moves require choosing an entire list. You can export a backup first.",
      ],
      [
        "If tasks seem to be missing",
        "Check your account, workspace, project archives, and Completed view. Do not clear browser storage while you have unsynced changes. Export all tasks from Settings first. Private browsing, storage cleanup, and browser eviction can remove locally saved data.",
      ],
    ],
  },
  {
    slug: "keyboard-shortcuts",
    title: "Less reaching for the mouse.",
    description: "A few useful shortcuts, without a system to memorize.",
    sections: [
      [
        "Search this workspace",
        "Ctrl+K focuses search. On a Mac keyboard in the web app, use Command+K. Search matches task titles and notes in the current workspace.",
      ],
      [
        "Add a task",
        "Type into the quick-add field and press Enter. Ctrl+Enter opens Task details when no editor is open. In the web app, Command+Enter works too.",
      ],
      [
        "Move through controls",
        "Tab and Shift+Tab move between controls. Enter or Space activates a focused button. Escape closes a dialog; CatDo asks before discarding an edited task. Browser Back and Forward return to previous workspace views.",
      ],
    ],
  },
  {
    slug: "privacy",
    title: "Where your tasks live.",
    description: "A straightforward explanation of CatDo’s data flow.",
    sections: [
      [
        "On your devices",
        "The web app saves your tasks in IndexedDB and caches the app files for offline use. It remembers the last account and your appearance preference in local storage. The Linux app saves tasks in its local database. Signing out does not erase saved task data.",
      ],
      [
        "With your account",
        "Clerk handles sign-in through your shared WorkerCat account. CatDo uses your verified account identifier to keep your task data separate from other accounts.",
      ],
      [
        "In sync storage",
        "Task data is sent over HTTPS to CatDo on Cloudflare and stored in an account-specific Durable Object. This supports syncing across devices. CatDo does not claim end-to-end encryption: the service processes task data to store and sync it.",
      ],
      [
        "Backups and questions",
        "Settings includes a JSON export of your tasks. Keep backups somewhere private. For security concerns, use the private vulnerability report linked in the footer.",
      ],
    ],
  },
] as const;
