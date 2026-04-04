+++
title = "Getting Started"
+++

This project is highly experimental and, at the moment, not super user friendly. It's a work in progress.

To get started, fork the repo, clone it to your local computer and build a release version of the project. After that, install the resulting binary in a location of your choice.

For more details, see the offical [Readme](https://github.com/crustyrustacean/taxus/blob/trunk/README.md)

## Content Creation

Once a site is scaffolded, you'll see a `content`directory. You'll see an `_index.md` file with starter content. This is matched up with the `section.html` template to produce the home page. To make more content, make some folders, name them what you want, and put an `_index.md` inside each. This will create a corresponding `index.html` page for each area of your site.

Templates can be overriden anytime. In the frontmatter section of `_index.html`, simply specify the name of your custom template and make sure to add it to the `templates` directory.

## Styling

Existing static site generators, such as Zola or Hugo, rely on a theming system that I've always found complex. Plus, if you chose a theme, you're now reliant on that author's choices and ability to keep it up to date.

I decided to make things simple. There's a `main.scss` scaffolded in the `styles` directory. Use it to style your site. Done.

## Hot Reloading

You can build your site incrementally with `taxus` development server. If you misstep, the browser and console will output error messages to help you zoom in on the issue.


