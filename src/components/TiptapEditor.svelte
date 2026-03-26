<script lang="ts">
  import { onMount, onDestroy, createEventDispatcher, tick } from 'svelte'
  import { Editor } from '@tiptap/core'
  import StarterKit from '@tiptap/starter-kit'
  import Placeholder from '@tiptap/extension-placeholder'
  import Underline from '@tiptap/extension-underline'
  import Highlight from '@tiptap/extension-highlight'
  import TaskList from '@tiptap/extension-task-list'
  import TaskItem from '@tiptap/extension-task-item'
  import Link from '@tiptap/extension-link'
  import BubbleMenu from '@tiptap/extension-bubble-menu'
  import Icon from '@iconify/svelte'
  import { SlashCommands } from '../lib/editor/slash-commands'
  import { storedContentToEditorHtml } from '../lib/editor/content'
  import 'tippy.js/dist/tippy.css'
  import '../lib/editor/editor-styles.css'

  /** Changes when switching documents so the editor reloads stored HTML. */
  export let documentKey = ''
  /** Stored DB content: HTML and/or legacy markdown/plain text. */
  export let html = ''
  export let placeholder = 'Start typing...'
  export let editable = true
  /** Extra class on the outer shell (e.g. sleek-content task-desc). */
  export let shellClass = ''

  const dispatch = createEventDispatcher<{ change: { html: string } }>()

  let mountEl: HTMLDivElement
  let bubbleEl: HTMLDivElement
  let editor: Editor | null = null
  let lastDocKey = ''
  /** Bumped on selection/transaction so toolbar active states recompute. */
  let toolbarTick = 0

  function applyLink() {
    if (!editor) return
    const href = typeof window !== 'undefined' ? window.prompt('Link URL') : null
    if (!href?.trim()) return
    const u = href.trim()
    if (editor.state.selection.empty) {
      editor
        .chain()
        .focus()
        .insertContent({ type: 'text', text: u, marks: [{ type: 'link', attrs: { href: u } }] })
        .run()
    } else {
      editor.chain().focus().setLink({ href: u }).run()
    }
  }

  onMount(async () => {
    await tick()
    if (!mountEl || !bubbleEl) return

    editor = new Editor({
      element: mountEl,
      editable,
      extensions: [
        StarterKit.configure({
          heading: { levels: [1, 2, 3] },
          bulletList: { keepMarks: true },
          orderedList: { keepMarks: true }
        }),
        Underline,
        Highlight.configure({ multicolor: true }),
        TaskList,
        TaskItem.configure({ nested: true }),
        Link.configure({ openOnClick: false, autolink: true }),
        Placeholder.configure({
          placeholder,
          emptyEditorClass: 'is-editor-empty',
          showOnlyWhenEditable: true
        }),
        SlashCommands,
        BubbleMenu.configure({
          element: bubbleEl
        })
      ],
      content: storedContentToEditorHtml(html),
      onUpdate: ({ editor: ed }) => {
        dispatch('change', { html: ed.getHTML() })
      },
      // Selection + doc changes both go through transactions — enough to refresh toolbar actives.
      onTransaction: () => {
        toolbarTick++
      }
    })
    lastDocKey = documentKey
  })

  onDestroy(() => {
    editor?.destroy()
    editor = null
  })

  $: if (editor && documentKey !== lastDocKey) {
    lastDocKey = documentKey
    editor.commands.setContent(storedContentToEditorHtml(html))
  }

  $: if (editor && editor.isEditable !== editable) {
    editor.setEditable(editable)
  }

  // `toolbarTick` is read so Svelte reruns this when selection/doc changes.
  $: toolbar = (() => {
    void toolbarTick
    if (!editor) return null
    return {
      bold: editor.isActive('bold'),
      italic: editor.isActive('italic'),
      underline: editor.isActive('underline'),
      highlight: editor.isActive('highlight'),
      link: editor.isActive('link'),
      h1: editor.isActive('heading', { level: 1 }),
      h2: editor.isActive('heading', { level: 2 }),
      h3: editor.isActive('heading', { level: 3 }),
      bullet: editor.isActive('bulletList'),
      ordered: editor.isActive('orderedList'),
      task: editor.isActive('taskList'),
      quote: editor.isActive('blockquote'),
      code: editor.isActive('code'),
      codeBlock: editor.isActive('codeBlock')
    }
  })()
</script>

<div class="kalam-tiptap-shell {shellClass}">
  {#if editor && toolbar}
    <div class="kalam-tiptap-toolbar" role="toolbar" aria-label="Text formatting">
      <button
        type="button"
        class="kalam-tiptap-toolbar__btn"
        class:active={toolbar.bold}
        title="Bold"
        aria-pressed={toolbar.bold}
        on:click={() => editor?.chain().focus().toggleBold().run()}
      >
        <Icon icon="ph:text-b" />
      </button>
      <button
        type="button"
        class="kalam-tiptap-toolbar__btn"
        class:active={toolbar.italic}
        title="Italic"
        aria-pressed={toolbar.italic}
        on:click={() => editor?.chain().focus().toggleItalic().run()}
      >
        <Icon icon="ph:text-italic" />
      </button>
      <button
        type="button"
        class="kalam-tiptap-toolbar__btn"
        class:active={toolbar.underline}
        title="Underline"
        aria-pressed={toolbar.underline}
        on:click={() => editor?.chain().focus().toggleUnderline().run()}
      >
        <Icon icon="ph:text-underline" />
      </button>
      <button
        type="button"
        class="kalam-tiptap-toolbar__btn"
        class:active={toolbar.highlight}
        title="Highlight"
        aria-pressed={toolbar.highlight}
        on:click={() => editor?.chain().focus().toggleHighlight().run()}
      >
        <Icon icon="ph:highlighter-circle" />
      </button>
      <button
        type="button"
        class="kalam-tiptap-toolbar__btn"
        class:active={toolbar.link}
        title="Link"
        aria-pressed={toolbar.link}
        on:click={applyLink}
      >
        <Icon icon="ph:link" />
      </button>
      <span class="kalam-tiptap-toolbar__sep" aria-hidden="true"></span>
      <button
        type="button"
        class="kalam-tiptap-toolbar__btn kalam-tiptap-toolbar__btn--text"
        class:active={toolbar.h1}
        title="Heading 1"
        aria-pressed={toolbar.h1}
        on:click={() => editor?.chain().focus().toggleHeading({ level: 1 }).run()}
      >
        H1
      </button>
      <button
        type="button"
        class="kalam-tiptap-toolbar__btn kalam-tiptap-toolbar__btn--text"
        class:active={toolbar.h2}
        title="Heading 2"
        aria-pressed={toolbar.h2}
        on:click={() => editor?.chain().focus().toggleHeading({ level: 2 }).run()}
      >
        H2
      </button>
      <button
        type="button"
        class="kalam-tiptap-toolbar__btn kalam-tiptap-toolbar__btn--text"
        class:active={toolbar.h3}
        title="Heading 3"
        aria-pressed={toolbar.h3}
        on:click={() => editor?.chain().focus().toggleHeading({ level: 3 }).run()}
      >
        H3
      </button>
      <span class="kalam-tiptap-toolbar__sep" aria-hidden="true"></span>
      <button
        type="button"
        class="kalam-tiptap-toolbar__btn"
        class:active={toolbar.bullet}
        title="Bullet list"
        aria-pressed={toolbar.bullet}
        on:click={() => editor?.chain().focus().toggleBulletList().run()}
      >
        <Icon icon="ph:list-bullets" />
      </button>
      <button
        type="button"
        class="kalam-tiptap-toolbar__btn"
        class:active={toolbar.ordered}
        title="Numbered list"
        aria-pressed={toolbar.ordered}
        on:click={() => editor?.chain().focus().toggleOrderedList().run()}
      >
        <Icon icon="ph:list-numbers" />
      </button>
      <button
        type="button"
        class="kalam-tiptap-toolbar__btn"
        class:active={toolbar.task}
        title="Task list"
        aria-pressed={toolbar.task}
        on:click={() => editor?.chain().focus().toggleTaskList().run()}
      >
        <Icon icon="ph:checks" />
      </button>
      <span class="kalam-tiptap-toolbar__sep" aria-hidden="true"></span>
      <button
        type="button"
        class="kalam-tiptap-toolbar__btn"
        class:active={toolbar.quote}
        title="Quote"
        aria-pressed={toolbar.quote}
        on:click={() => editor?.chain().focus().toggleBlockquote().run()}
      >
        <Icon icon="ph:quotes" />
      </button>
      <button
        type="button"
        class="kalam-tiptap-toolbar__btn"
        class:active={toolbar.code}
        title="Inline code"
        aria-pressed={toolbar.code}
        on:click={() => editor?.chain().focus().toggleCode().run()}
      >
        <Icon icon="ph:code" />
      </button>
      <button
        type="button"
        class="kalam-tiptap-toolbar__btn"
        class:active={toolbar.codeBlock}
        title="Code block"
        aria-pressed={toolbar.codeBlock}
        on:click={() => editor?.chain().focus().toggleCodeBlock().run()}
      >
        <Icon icon="ph:brackets-curly" />
      </button>
      <button
        type="button"
        class="kalam-tiptap-toolbar__btn"
        title="Horizontal rule"
        on:click={() => editor?.chain().focus().setHorizontalRule().run()}
      >
        <Icon icon="ph:minus" />
      </button>
    </div>
  {/if}
  <div class="kalam-tiptap-mount" bind:this={mountEl}></div>
  <div class="kalam-bubble-menu" bind:this={bubbleEl} aria-hidden="true">
    <button type="button" title="Bold" on:click={() => editor?.chain().focus().toggleBold().run()}>B</button>
    <button type="button" title="Italic" on:click={() => editor?.chain().focus().toggleItalic().run()}>I</button>
    <button type="button" title="Underline" on:click={() => editor?.chain().focus().toggleUnderline().run()}>U</button>
    <button type="button" title="Highlight" on:click={() => editor?.chain().focus().toggleHighlight().run()}>H</button>
    <button type="button" title="Link" on:click={applyLink}>Link</button>
  </div>
</div>
