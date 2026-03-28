/**
 * Kalam Website Shared JavaScript
 * Handles theme toggle, navigation, focus management, and accessibility
 */

(function() {
    'use strict';

    // ===== SHOWCASE THEME (for index.html) =====
    window.setShowcaseTheme = function(mode) {
        var images = document.querySelectorAll('#showcase-grid img[data-light]');
        images.forEach(function(img) {
            img.src = mode === 'dark' ? img.getAttribute('data-dark') : img.getAttribute('data-light');
        });
    };

    // ===== THEME MANAGEMENT =====
    var themeToggle = document.getElementById('theme-toggle');
    var themeIcon = document.getElementById('theme-icon');

    function setTheme(theme) {
        document.documentElement.setAttribute('data-theme', theme);
        localStorage.setItem('kalam-site-theme', theme);
        if (themeIcon) {
            themeIcon.classList.toggle('ri-sun-line', theme === 'light');
            themeIcon.classList.toggle('ri-moon-line', theme === 'dark');
        }
        // Update showcase screenshots if function exists
        if (typeof setShowcaseTheme === 'function') {
            setShowcaseTheme(theme);
        }
    }

    // Initialize theme icon on page load
    if (document.documentElement.getAttribute('data-theme') === 'light' && themeIcon) {
        themeIcon.classList.remove('ri-moon-line');
        themeIcon.classList.add('ri-sun-line');
    }

    if (themeToggle) {
        themeToggle.addEventListener('click', function() {
            var currentTheme = document.documentElement.getAttribute('data-theme');
            var newTheme = currentTheme === 'dark' ? 'light' : 'dark';
            setTheme(newTheme);
        });
    }

    // Make setTheme available globally for showcase
    window.setTheme = setTheme;

    // ===== MOBILE NAVIGATION WITH FOCUS TRAP =====
    var navToggle = document.getElementById('nav-toggle');
    var navToggleIcon = document.getElementById('nav-toggle-icon');
    var siteNav = document.getElementById('site-nav');
    var focusableElementsString = 'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';
    var navFocusTrap = null;

    function getFocusableElements(container) {
        return Array.prototype.slice.call(container.querySelectorAll(focusableElementsString));
    }

    function trapFocus(container, returnElement) {
        var focusableElements = getFocusableElements(container);
        var firstFocusable = focusableElements[0];
        var lastFocusable = focusableElements[focusableElements.length - 1];

        function handleKeyDown(e) {
            if (e.key !== 'Tab') return;

            if (e.shiftKey) {
                if (document.activeElement === firstFocusable) {
                    e.preventDefault();
                    lastFocusable.focus();
                }
            } else {
                if (document.activeElement === lastFocusable) {
                    e.preventDefault();
                    firstFocusable.focus();
                }
            }
        }

        container.addEventListener('keydown', handleKeyDown);

        // Store reference to remove listener later
        return {
            destroy: function() {
                container.removeEventListener('keydown', handleKeyDown);
                if (returnElement) {
                    returnElement.focus();
                }
            },
            focusFirst: function() {
                if (firstFocusable) {
                    firstFocusable.focus();
                }
            }
        };
    }

    if (navToggle && siteNav) {
        navToggle.addEventListener('click', function() {
            var isOpen = siteNav.classList.toggle('open');

            if (navToggleIcon) {
                navToggleIcon.classList.toggle('ri-menu-line', !isOpen);
                navToggleIcon.classList.toggle('ri-close-line', isOpen);
            }

            // Prevent body scroll while menu is open
            document.body.style.overflow = isOpen ? 'hidden' : '';

            // Manage focus trap
            if (isOpen) {
                navFocusTrap = trapFocus(siteNav, navToggle);
                // Move focus to first nav link after animation
                setTimeout(function() {
                    if (navFocusTrap) navFocusTrap.focusFirst();
                }, 100);
            } else {
                if (navFocusTrap) {
                    navFocusTrap.destroy();
                    navFocusTrap = null;
                }
            }
        });

        // Close nav on escape key
        document.addEventListener('keydown', function(e) {
            if (e.key === 'Escape' && siteNav.classList.contains('open')) {
                siteNav.classList.remove('open');
                document.body.style.overflow = '';
                if (navToggleIcon) {
                    navToggleIcon.classList.add('ri-menu-line');
                    navToggleIcon.classList.remove('ri-close-line');
                }
                if (navFocusTrap) {
                    navFocusTrap.destroy();
                    navFocusTrap = null;
                }
                navToggle.focus();
            }
        });

        // Close nav when clicking a link
        var navLinks = siteNav.querySelectorAll('a');
        navLinks.forEach(function(link) {
            link.addEventListener('click', function() {
                if (siteNav.classList.contains('open')) {
                    siteNav.classList.remove('open');
                    document.body.style.overflow = '';
                    if (navToggleIcon) {
                        navToggleIcon.classList.add('ri-menu-line');
                        navToggleIcon.classList.remove('ri-close-line');
                    }
                    if (navFocusTrap) {
                        navFocusTrap.destroy();
                        navFocusTrap = null;
                    }
                }
            });
        });
    }

    // ===== SCROLL REVEAL ANIMATIONS =====
    function initScrollReveal() {
        var reveals = document.querySelectorAll('.reveal');

        function checkReveals() {
            var wh = window.innerHeight;
            reveals.forEach(function(r) {
                var rect = r.getBoundingClientRect();
                r.style.animationPlayState = rect.top < wh - 100 ? 'running' : 'paused';
            });
        }

        // Pause animations for elements below fold on load
        reveals.forEach(function(r) {
            if (r.getBoundingClientRect().top > window.innerHeight) {
                r.style.animationPlayState = 'paused';
            }
        });

        window.addEventListener('scroll', checkReveals, { passive: true });
        checkReveals(); // Initial check
    }

    // ===== ACCORDION FUNCTIONALITY =====
    function initAccordions() {
        document.querySelectorAll('.accordion-header').forEach(function(header) {
            header.addEventListener('click', function() {
                var accordion = header.parentElement;
                var wasOpen = accordion.classList.contains('open');

                // Close all accordions (optional - remove if multiple open is desired)
                document.querySelectorAll('.accordion').forEach(function(a) {
                    a.classList.remove('open');
                });

                if (!wasOpen) {
                    accordion.classList.add('open');
                }
            });

            // Add keyboard support
            header.addEventListener('keydown', function(e) {
                if (e.key === 'Enter' || e.key === ' ') {
                    e.preventDefault();
                    header.click();
                }
            });

            // Make header focusable
            header.setAttribute('tabindex', '0');
            header.setAttribute('role', 'button');
            header.setAttribute('aria-expanded', 'false');

            // Update aria-expanded on toggle
            var accordion = header.parentElement;
            var observer = new MutationObserver(function(mutations) {
                mutations.forEach(function(mutation) {
                    if (mutation.attributeName === 'class') {
                        var isOpen = accordion.classList.contains('open');
                        header.setAttribute('aria-expanded', isOpen ? 'true' : 'false');
                    }
                });
            });
            observer.observe(accordion, { attributes: true });
        });
    }

    // ===== LIGHTBOX FOCUS MANAGEMENT =====
    var lightboxFocusTrap = null;

    window.openShowcaseLightbox = function(element) {
        var img = element.querySelector('img');
        var lightbox = document.getElementById('showcase-lightbox');
        var lightboxImg = document.getElementById('showcase-lightbox-img');
        var closeBtn = lightbox.querySelector('.showcase-lightbox-close');

        if (!img || !lightbox || !lightboxImg) return;

        // Store reference to triggering element
        lightbox.dataset.triggerElement = element;

        lightboxImg.src = img.src;
        lightboxImg.alt = img.alt || 'Screenshot';
        lightbox.classList.add('open');
        document.body.style.overflow = 'hidden';

        // Set up focus trap
        lightboxFocusTrap = trapFocus(lightbox, closeBtn);

        // Move focus to close button after animation
        setTimeout(function() {
            if (closeBtn) closeBtn.focus();
        }, 100);
    };

    window.closeShowcaseLightbox = function() {
        var lightbox = document.getElementById('showcase-lightbox');
        if (!lightbox) return;

        lightbox.classList.remove('open');
        document.body.style.overflow = '';

        // Return focus to triggering element
        var triggerElement = lightbox.dataset.triggerElement;
        if (triggerElement) {
            triggerElement.focus();
        }

        if (lightboxFocusTrap) {
            lightboxFocusTrap.destroy();
            lightboxFocusTrap = null;
        }
    };

    // Escape key for lightbox
    document.addEventListener('keydown', function(e) {
        if (e.key === 'Escape') {
            var lightbox = document.getElementById('showcase-lightbox');
            if (lightbox && lightbox.classList.contains('open')) {
                window.closeShowcaseLightbox();
            }
        }
    });

    // Make showcase cards focusable and keyboard accessible
    function initShowcaseCards() {
        document.querySelectorAll('.showcase-card').forEach(function(card) {
            card.setAttribute('tabindex', '0');
            card.setAttribute('role', 'button');
            card.setAttribute('aria-label', 'View enlarged screenshot');

            card.addEventListener('keydown', function(e) {
                if (e.key === 'Enter' || e.key === ' ') {
                    e.preventDefault();
                    window.openShowcaseLightbox(card);
                }
            });
        });
    }

    // ===== SKIP TO CONTENT =====
    // Handle skip link target focus
    var skipLink = document.querySelector('.skip-to-content');
    var mainContent = document.getElementById('main-content');

    if (skipLink && mainContent) {
        skipLink.addEventListener('click', function(e) {
            e.preventDefault();
            mainContent.setAttribute('tabindex', '-1');
            mainContent.focus();
            mainContent.removeAttribute('tabindex');
        });
    }

    // ===== INITIALIZATION =====
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', function() {
            initScrollReveal();
            initAccordions();
            initShowcaseCards();
        });
    } else {
        initScrollReveal();
        initAccordions();
        initShowcaseCards();
    }

})();
