import { Extension } from '@tiptap/core'
import type { Editor, Range } from '@tiptap/core'
import Suggestion, { type SuggestionProps } from '@tiptap/suggestion'
import { PluginKey } from '@tiptap/pm/state'
import tippy, { type Instance as TippyInstance } from 'tippy.js'

export type SlashMenuItem = {
  title: string
  description: string
  keywords: string[]
  command: (opts: { editor: Editor; range: Range }) => void
}

function buildSlashItems(): SlashMenuItem[] {
  return [
    {
      title: 'Heading 1',
      description: 'Large section heading',
      keywords: ['h1', 'title'],
      command: ({ editor, range }) => {
        editor.chain().focus().deleteRange(range).setHeading({ level: 1 }).run()
      }
    },
    {
      title: 'Heading 2',
      description: 'Medium section heading',
      keywords: ['h2', 'subtitle'],
      command: ({ editor, range }) => {
        editor.chain().focus().deleteRange(range).setHeading({ level: 2 }).run()
      }
    },
    {
      title: 'Bold',
      description: 'Bold text',
      keywords: ['strong'],
      command: ({ editor, range }) => {
        editor.chain().focus().deleteRange(range).toggleBold().run()
      }
    },
    {
      title: 'Italic',
      description: 'Italic text',
      keywords: ['emphasis'],
      command: ({ editor, range }) => {
        editor.chain().focus().deleteRange(range).toggleItalic().run()
      }
    },
    {
      title: 'Underline',
      description: 'Underline text',
      keywords: ['u'],
      command: ({ editor, range }) => {
        editor.chain().focus().deleteRange(range).toggleUnderline().run()
      }
    },
    {
      title: 'Bullet list',
      description: 'Unordered list',
      keywords: ['ul', 'unordered'],
      command: ({ editor, range }) => {
        editor.chain().focus().deleteRange(range).toggleBulletList().run()
      }
    },
    {
      title: 'Numbered list',
      description: 'Ordered list',
      keywords: ['ol', 'ordered'],
      command: ({ editor, range }) => {
        editor.chain().focus().deleteRange(range).toggleOrderedList().run()
      }
    },
    {
      title: 'Task list',
      description: 'Checkboxes',
      keywords: ['todo', 'checkbox'],
      command: ({ editor, range }) => {
        editor.chain().focus().deleteRange(range).toggleTaskList().run()
      }
    },
    {
      title: 'Highlight',
      description: 'Marker highlight',
      keywords: ['mark', 'yellow'],
      command: ({ editor, range }) => {
        editor.chain().focus().deleteRange(range).toggleHighlight().run()
      }
    },
    {
      title: 'Code block',
      description: 'Preformatted code',
      keywords: ['pre'],
      command: ({ editor, range }) => {
        editor.chain().focus().deleteRange(range).toggleCodeBlock().run()
      }
    },
    {
      title: 'Quote',
      description: 'Blockquote',
      keywords: ['blockquote'],
      command: ({ editor, range }) => {
        editor.chain().focus().deleteRange(range).toggleBlockquote().run()
      }
    },
    {
      title: 'Divider',
      description: 'Horizontal rule',
      keywords: ['hr', 'horizontal', 'line'],
      command: ({ editor, range }) => {
        editor.chain().focus().deleteRange(range).setHorizontalRule().run()
      }
    },
    {
      title: 'Link',
      description: 'Add link URL',
      keywords: ['url', 'href'],
      command: ({ editor, range }) => {
        const href = typeof window !== 'undefined' ? window.prompt('Link URL') : null
        if (!href?.trim()) return
        const u = href.trim()
        const start = range.from
        editor
          .chain()
          .focus()
          .deleteRange(range)
          .insertContent({ type: 'text', text: u, marks: [{ type: 'link', attrs: { href: u } }] })
          .run()
      }
    }
  ]
}

function filterItems(query: string): SlashMenuItem[] {
  const items = buildSlashItems()
  const q = query.trim().toLowerCase()
  if (!q) return items
  return items.filter((item) => {
    const hay = `${item.title} ${item.description} ${item.keywords.join(' ')}`.toLowerCase()
    return hay.includes(q)
  })
}

/**
 * "/" suggestion menu: filters commands as the user types after `/`.
 */
export const SlashCommands = Extension.create({
  name: 'slashCommands',

  addProseMirrorPlugins() {
    const editor = this.editor
    const pluginKey = new PluginKey('kalamSlashMenu')
    let popup: TippyInstance | null = null
    let menuEl: HTMLDivElement | null = null
    let selectedIndex = 0
    let latest: SuggestionProps<SlashMenuItem, SlashMenuItem> | null = null

    function destroyPopup() {
      popup?.destroy()
      popup = null
      menuEl = null
      latest = null
      selectedIndex = 0
    }

    function renderMenu() {
      if (!menuEl || !latest) return
      menuEl.innerHTML = ''
      const items = latest.items
      items.forEach((item, index) => {
        const row = document.createElement('button')
        row.type = 'button'
        row.className = 'kalam-slash-menu__item'
        if (index === selectedIndex) row.classList.add('kalam-slash-menu__item--active')
        row.innerHTML = `<span class="kalam-slash-menu__title">${escapeAttr(item.title)}</span><span class="kalam-slash-menu__desc">${escapeAttr(item.description)}</span>`
        row.addEventListener('mousedown', (e) => e.preventDefault())
        row.addEventListener('click', () => {
          latest?.command(item)
        })
        menuEl.appendChild(row)
      })
    }

    return [
      Suggestion<SlashMenuItem, SlashMenuItem>({
        pluginKey,
        editor,
        char: '/',
        allowSpaces: true,
        startOfLine: false,
        items: ({ query }) => filterItems(query),
        command: ({ editor, range, props }) => {
          props.command({ editor, range })
        },
        render: () => ({
          onStart: (props) => {
            latest = props
            selectedIndex = 0
            menuEl = document.createElement('div')
            menuEl.className = 'kalam-slash-menu'
            menuEl.setAttribute('role', 'listbox')
            renderMenu()
            // theme `kalam-slash` — outer box stays transparent; real surface is `.kalam-slash-menu` in CSS
            popup = tippy(document.body, {
              getReferenceClientRect: () => props.clientRect?.() ?? new DOMRect(0, 0, 0, 0),
              appendTo: () => document.body,
              content: menuEl,
              showOnCreate: true,
              interactive: true,
              trigger: 'manual',
              placement: 'bottom-start',
              arrow: false,
              offset: [0, 6],
              theme: 'kalam-slash'
            })
          },
          onUpdate: (props) => {
            latest = props
            selectedIndex = Math.min(selectedIndex, Math.max(0, props.items.length - 1))
            renderMenu()
            popup?.setProps({
              getReferenceClientRect: () => props.clientRect?.() ?? new DOMRect(0, 0, 0, 0)
            })
          },
          onKeyDown: ({ event }) => {
            if (!latest?.items.length) return false
            if (event.key === 'ArrowDown') {
              selectedIndex = (selectedIndex + 1) % latest.items.length
              renderMenu()
              return true
            }
            if (event.key === 'ArrowUp') {
              selectedIndex = (selectedIndex - 1 + latest.items.length) % latest.items.length
              renderMenu()
              return true
            }
            if (event.key === 'Enter') {
              const item = latest.items[selectedIndex]
              if (item) latest.command(item)
              return true
            }
            return false
          },
          onExit: () => {
            destroyPopup()
          }
        })
      })
    ]
  }
})

function escapeAttr(s: string): string {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/"/g, '&quot;')
}
