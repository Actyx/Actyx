# developer.actyx.com

This directory contains the source used to build the [https://developer.actyx.com](https://developers.actyx.com) website. For more information about **contributing content**, please jump to [contributing content](#contributing-content). If you are interested in **how it works** or **developing** the site, please jump to [development](#development).

## Contributing content

> Please test before merging
>
> If you have contributed content, please test before merging to master. You can either use the netlify preview build (link in your Github PR's checks section) or build yourself. A merge to master means that the actual production site is updated.

### Content structure

The site has four sections:

#### ActyxOS

Documentation related to ActyxOS. All content can be found in `/docs/os`.

The sidebar (left on the page) is defined in the `osSidebar` property in `/sidebars.js`.

#### Actyx Pond

Documentation related to Actyx Pond. All content can be found in `/docs/pond`.

The sidebar (left on the page) is defined in the `pondSidebar` property in `/sidebars.js`.

#### Actyx CLI

Documentation related to the Actyx CLI. All content can be found in `/docs/cli.md` (currently it is just a single page).

The sidebar (left on the page) is defined in the `pondSidebar` property in `/sidebars.js`.

#### FAQ

Frequently asked questions. All content can be found in `/docs/faq`.

The sidebar (left on the page) is defined in the `faqSidebar` property in `/sidebars.js`.

### Content format

Docusaurus uses standard Markdown syntax extended with MDX. Any page should have the following structure

```
---
property1: value
property2: value
---

My content

## My first header

More content

## My second header

Even more content
```

As you can see, a page is composed of so-called _header fields_ (enclosed by `---` above and below), followed by the actual content. The content can be any valid Markdown. Indeed, you can even use MDX. Please refer to [this page](https://v2.docusaurus.io/docs/markdown-features/) for more information. 

**Important**: do not use h1 headers (i.e. `#`). The highest-level header on the page should be a second-level header (`##`).

#### Header fields

At the very least, each page should have a `title` header field.

#### Linking to other pages

Link to other pages using relative links and include the `.md` file extension (it will automatically be removed at build time). Example:

```md
Check out [this page](../pond/design-principles.md).
```

#### Images (and other static files)

Images and other static files must be places in the `/static` directory. If you place a file called `my-img.png` in `/static/images`, you can link to it within the markdown pages as follows:


```md
Here comes an image:

![](/images/my-img.png)
```

Note that the path does not include `static`. The fact that there is a difference between the path in the source and in the build, unfortunately means that preview doesn't show the images. You can solve this by creating a symlink in the root directory of the site as follows:

```
$ ln -s /static/images /images
```

#### Callouts

You can create _info_, _tip_, _note_ and _warning_ callouts using the following syntax (change `tip` to any of `info`, `tip`, `note`, `warning`).

```md
:::tip Title
The content and title *can* include markdown.
:::
```

### Changing existing pages

If you want to change the content of existing pages, simply find the corresponding Markdown file in `/docs` and edit accordingly. Then either wait for the preview build on your PR, or start the development server for testing or build the site for testing.

**Please verify that the content shows as expected before merging to master.**

### Adding new pages

In order to add a new page you must do two things:

1. Create the Markdown file
1. Add the correct header field(s)
1. Add the page to a corresponding sidebar.

If you want to add new sidebars, or create new sections either (a) figure it out using the existing code or the [Docusaurus (v2) documentation](https://v2.docusaurus.io/docs/introduction); or (b) ask OST.

#### Create the page

Create a new Markdown file in the correct directory, e.g. `/docs/os/further-information/great-question.md`.

#### Add required or optional header fields

At the top of the file add, at least, the following:

```
---
title: Great question.
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

```
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

### How it works

This website is built using [Docusaurus 2](https://v2.docusaurus.io/) with the `@docusaurus/preset-classic` preset. Docusaurus generates a static site that can be served from somewhere. We use Netlify, which automatically pulls `Cosmos` from Github, then builds and then serves the site at [https://developer.actyx.com](https://developer.actyx.com).

> Want to know more?
>
> OST set this up, so ping him if you would like to or need to know more.

### Developing

Set the correct npm version and install dependencies:

```
$ nvm use
$ npm install
```

Start the local development server (with hot reload):

```
$ npm run start
```

Build the site (output in `build/`):

```
$ npm run build
```
