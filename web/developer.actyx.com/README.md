# Actyx Developer Documentation

This directory contains the source used to build the [https://developer.actyx.com](https://developers.actyx.com) website. For more information about **contributing content**, please jump to [contributing content](#contributing-content). If you are interested in **how it works** or **developing** the site, please jump to [development](#development).

## Contributing content

> Please test before merging
>
> If you have contributed content, please test before merging to master. You can either use the netlify preview build (link in your Github PR's checks section) or build yourself. A merge to master means that the actual production site is updated.

### Content structure

The content of our docs is grouped by 4 categories:

#### Conceptual Guides

These <b>understanding-oriented</b> guides clarify a particular topic by giving context and a wider view.

#### How-to Guides

These <b>problem-oriented</b> guides take the reader through a series of steps required to solve a problem.

#### API Reference Docs

These <b>information-oriented</b> docs provide technical descriptions of the code and how to operate it.

#### Tutorials

These <b>learning-oriented</b> lessons take the reader by the hand to complete a small project. Note that all the tutorials are located in the Actyx Academy and ownership lies entirely with Developer Advocacy. The responsibility with the rest of the content lies within Product Management.

### Markdown Pages

A page is composed of so-called _frontmatter_ (enclosed by `---` above and below), followed by the actual content. Please make sure you fill in all fields in the frontmatter section. The content can be any valid Markdown. Indeed, you can even use MDX. Please refer to [this page](https://v2.docusaurus.io/docs/markdown-features/) for more information.

#### Linking to other pages

Link to other pages using relative links and include the `.md` file extension (it will automatically be removed at build time). Example:

```md
Check out [this page](../pond/design-principles.md).
```

#### Images (and other static files)

Images and other static files must be places in the `/static` directory. If you place a file called `my-img.png` in `/static/images`, you can link to it within the markdown pages as follows:

```md
Here comes an image:

![My image](/images/my-img.png)
```

Note that the path does not include `static`. The fact that there is a difference between the path in the source and in the build, unfortunately means that preview doesn't show the images. You can solve this by creating a symlink in the root directory of the site as follows:

```bash
ln -s /static/images /images
```

All .svg image files that are in the static or any inner folder will be optimized during `prebuild` using an [svg optimizer](https://www.npmjs.com/package/svgo) and saved back under their original name in the same folder.

#### Callouts

You can create _info_, _tip_, _note_ and _warning_ callouts using the following syntax (change `tip` to any of `info`, `tip`, `note`, `warning`).

```md
:::tip Title
The content and title _can_ include markdown.
:::
```

### Changing existing pages

If you want to change the content of existing pages, simply find the corresponding Markdown file in `/docs` and edit accordingly. Then either wait for the preview build on your PR, or start the development server for testing or build the site for testing.

**Please verify that the content shows as expected before merging to master.**

### Adding new pages

In order to add a new page you must do two things:

1. Create the Markdown file
1. Add the correct front matter fields
1. Add the page to a corresponding sidebar.

If you want to add new sidebars, or create new sections either (a) figure it out using the existing code or the [Docusaurus (v2) documentation](https://v2.docusaurus.io/docs/introduction); or (b) ask Product Management.

#### Create the page

Create a new `.mdx` file in the correct directory, e.g. `/docs/conceptual-guides/`.

#### Add required or optional frontmatter

At the top of the file add, at least, the following:

```md
---
title: Title
id: title
hide_title: false
hide_table_of_contents: false
sidebar_label: Sidebar Title
keywords: [some, fitting, keywords]
description: Some description of the content of this document. This description will be shown in thumbnails when for example posting on Twitter.
image: /images/os/js-sdk.png
---
```

Here are the other header fields you can use (copy pasted from [the Docusaurus documentation](https://v2.docusaurus.io/docs/markdown-features/#markdown-headers)):

- `id`: A unique document id. If this field is not present, the document's id will default to its file name (without the extension).
- `title`: The title of your document. If this field is not present, the document's title will default to its id.
- `hide_title`: Whether to hide the title at the top of the doc. By default it is false.
- `hide_table_of_contents`: Whether to hide the table of contents to the right. By default it is false.
- `sidebar_label`: The text shown in the document sidebar and in the next/previous button for this document. If this field is not present, the document's sidebar_label will default to its title.
- `custom_edit_url`: The URL for editing this document. If this field is not present, the document's edit URL will fall back to editUrl from options fields passed to docusaurus-plugin-content-docs.
- `keywords`: Keywords meta tag for the document page, for search engines.
- `description`: The description of your document, which will become the <meta name="description" content="..."/> and <meta property="og:description" content="..."/> in <head>, used by search engines. If this field is not present, it will default to the first line of the contents.
- `image`: Cover or thumbnail image that will be used when displaying the link to your post.

#### Add the page to a corresponding sidebar

In the `/sidebars.js` file, add the corresponding path to the correct sidebar. E.g. if you wanted to add a new section and link to the Actyx Pond docs, you would do so as follows:

```json
  pondSidebar: {
    'Actyx Pond': [
      'pond/introduction',
      'pond/design-principles',
    ],
    'Getting Started': [
      'pond/getting-started/installation',
    ],
    'Guides': [
      'pond/guides/hello-world',
      'pond/guides/events',
      'pond/guides/local-state',
      'pond/guides/subscriptions',
      'pond/guides/time-travel',
      'pond/guides/state-effects',
      'pond/guides/types',
      'pond/guides/snapshots',
      'pond/guides/integrating-a-ui',
    ],
    'NEW SECTION': [ // <- Add a new section
        'path/to/new/page', // <- Add a new link to the side bar
    ]
  },
```

Please check the existing sections and links in `/sidebars.js` for examples.

### React Components

To extend the capabilities of docusaurus, there are also a couple of react components to choose from when building pages. The components are:

- `SectionHero.tsx`
  This element has an optional button (`showButton={false}` prevents displaying the button)
- `TwoElementRow.tsx`
  The two col element can have 0-n optional links and 0-n optional tags
- `ThreeElementRow.tsx`
  This element can have 0-n optional links
- `DownloadLink.tsx`
- `StayInformed.tsx`

### Redirects

There is a `_redirects` file in `/netlify/` which defines site redirects. If you change the folder structure of the docs in a PR so that links might now lead to 404s (from `/docs/quickstart` to `/docs/learn-actyx/quickstart`), please make sure to include the FROM url and the TO url separated by a space in the `_redirects` file.

### How it works

This website is built using [Docusaurus 2](https://v2.docusaurus.io/) with the `@docusaurus/preset-classic` preset. Docusaurus generates a static site that can be served from somewhere. We use Netlify, which automatically pulls `Cosmos` from Github, then builds and then serves the site at [https://developer.actyx.com](https://developer.actyx.com).

> Want to know more?
>
> OST set this up, so ping him if you would like to or need to know more.

### Developing

Set the correct npm version and install dependencies:

```bash
nvm use
npm install
```

Start the local development server (with hot reload):

```bash
npm run start
```

Build the site (output in `build/`):

```bash
npm run build
```

#### Linting

Automatically detect common mistakes in markdown files, like syntax and grammar issues:

```bash
npm run lint
```

Automatically fix markdown issues found where possible:

```bash
npm run lint:md:fix
```

Automatically fix text issues found where possible:

```bash
npm run lint:txt:fix
```

Fix with dry-run (preview):

```bash
npm run lint:txt:dry
```

For a better linter experience when typing, please use this vscode extension
<https://marketplace.visualstudio.com/items?itemName=DavidAnson.vscode-markdownlint>

```bash
code --install-extension DavidAnson.vscode-markdownlint
```

You can disable some rules using:

<!-- markdownlint-disable MD037 -->

Some problematic text ([Lamport time](https://en.wikipedia.org/wiki/Lamport_timestamp))

<!-- markdownlint-enable MD037 -->

More information about markdownlint configuration can be found here:
<https://github.com/DavidAnson/markdownlint#configuration>
