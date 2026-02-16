export class SelectEnhancer {
    constructor(uiManager) {
        this.uiManager = uiManager;
        this.instances = new Map();

        document.addEventListener('click', (event) => this._handleDocumentClick(event));
        document.addEventListener('keydown', (event) => this._handleDocumentKeydown(event));
        window.addEventListener('blur', () => this.closeAll());
        window.addEventListener('resize', () => this.closeAll());
    }

    enhance(selectId, options = {}) {
        const select = document.getElementById(selectId);
        if (!select) return null;

        if (this.instances.has(selectId)) {
            this.refresh(selectId);
            return this.instances.get(selectId);
        }

        const wrapper = document.createElement('div');
        wrapper.className = 'ux-select';
        wrapper.dataset.selectId = selectId;
        if (options.variant) {
            wrapper.dataset.variant = options.variant;
        }

        const trigger = document.createElement('button');
        trigger.type = 'button';
        trigger.className = 'ux-select-trigger';
        trigger.setAttribute('aria-haspopup', 'listbox');
        trigger.setAttribute('aria-expanded', 'false');
        trigger.setAttribute('aria-controls', `${selectId}-ux-menu`);
        trigger.innerHTML = `
            <span class="ux-select-label"></span>
            <span class="ux-select-caret">▾</span>
        `;

        const menu = document.createElement('div');
        menu.id = `${selectId}-ux-menu`;
        menu.className = 'ux-select-menu themed-scrollbar-compact';
        menu.setAttribute('role', 'listbox');

        wrapper.appendChild(trigger);
        wrapper.appendChild(menu);

        select.classList.add('ux-select-native');
        select.insertAdjacentElement('afterend', wrapper);

        const instance = {
            selectId,
            select,
            wrapper,
            trigger,
            menu,
            options,
            open: false,
            highlightedIndex: -1,
            observer: null
        };

        trigger.addEventListener('click', (event) => {
            event.stopPropagation();
            if (instance.open) {
                this.close(selectId);
            } else {
                this.open(selectId);
            }
        });

        trigger.addEventListener('keydown', (event) => {
            this._handleTriggerKeydown(instance, event);
        });

        menu.addEventListener('mousedown', (event) => {
            event.preventDefault();
        });

        menu.addEventListener('click', (event) => {
            const optionButton = event.target.closest('.ux-select-option');
            if (!optionButton || optionButton.disabled) return;
            this._setValue(instance, optionButton.dataset.value, true);
        });

        menu.addEventListener('mousemove', (event) => {
            const optionButton = event.target.closest('.ux-select-option');
            if (!optionButton) return;
            const index = Number(optionButton.dataset.index);
            if (!Number.isNaN(index)) {
                this._setHighlightedIndex(instance, index, false);
            }
        });

        select.addEventListener('change', () => {
            this.sync(selectId);
        });

        const observer = new MutationObserver(() => {
            this.refresh(selectId);
        });

        observer.observe(select, {
            childList: true,
            subtree: true,
            characterData: true,
            attributes: true,
            attributeFilter: ['disabled', 'label']
        });

        instance.observer = observer;
        this.instances.set(selectId, instance);
        this.refresh(selectId);
        return instance;
    }

    refresh(selectId) {
        const instance = this.instances.get(selectId);
        if (!instance) return;

        const options = Array.from(instance.select.options);
        instance.menu.innerHTML = '';

        options.forEach((option, index) => {
            const optionButton = document.createElement('button');
            optionButton.type = 'button';
            optionButton.className = 'ux-select-option';
            optionButton.setAttribute('role', 'option');
            optionButton.dataset.value = option.value;
            optionButton.dataset.index = String(index);
            optionButton.textContent = option.textContent || option.value;

            if (instance.selectId === 'log-level-select') {
                const normalizedValue = String(option.value || '').toUpperCase();
                if (normalizedValue) {
                    optionButton.classList.add(`level-${normalizedValue.toLowerCase()}`);
                }
            }

            if (option.disabled) {
                optionButton.disabled = true;
            }
            instance.menu.appendChild(optionButton);
        });

        this.sync(selectId);
    }

    sync(selectId) {
        const instance = this.instances.get(selectId);
        if (!instance) return;

        const options = Array.from(instance.select.options);
        const selectedIndex = instance.select.selectedIndex >= 0 ? instance.select.selectedIndex : 0;
        const selectedOption = options[selectedIndex];
        const labelElement = instance.trigger.querySelector('.ux-select-label');
        if (labelElement) {
            labelElement.textContent = selectedOption
                ? (selectedOption.textContent || '')
                : (instance.options.placeholder || 'Select');
        }

        instance.wrapper.dataset.value = instance.select.value || '';

        const optionButtons = Array.from(instance.menu.querySelectorAll('.ux-select-option'));
        optionButtons.forEach((button, index) => {
            const isActive = index === selectedIndex;
            button.classList.toggle('active', isActive);
            button.setAttribute('aria-selected', isActive ? 'true' : 'false');
        });

        const isDisabled = !!instance.select.disabled;
        instance.wrapper.classList.toggle('is-disabled', isDisabled);
        instance.trigger.disabled = isDisabled;

        if (!instance.open) {
            instance.highlightedIndex = selectedIndex;
        }
    }

    open(selectId) {
        const instance = this.instances.get(selectId);
        if (!instance || instance.select.disabled) return;

        this.closeAll(selectId);
        instance.open = true;
        instance.wrapper.classList.add('open');
        instance.trigger.setAttribute('aria-expanded', 'true');

        this.sync(selectId);
        const selectedIndex = instance.select.selectedIndex >= 0 ? instance.select.selectedIndex : 0;
        this._setHighlightedIndex(instance, selectedIndex, true);
    }

    close(selectId) {
        const instance = this.instances.get(selectId);
        if (!instance || !instance.open) return;

        instance.open = false;
        instance.wrapper.classList.remove('open');
        instance.trigger.setAttribute('aria-expanded', 'false');
    }

    closeAll(exceptSelectId = null) {
        this.instances.forEach((instance) => {
            if (exceptSelectId && instance.selectId === exceptSelectId) return;
            this.close(instance.selectId);
        });
    }

    _setValue(instance, value, closeMenu) {
        if (instance.select.value !== value) {
            instance.select.value = value;
            instance.select.dispatchEvent(new Event('change', { bubbles: true }));
        } else {
            this.sync(instance.selectId);
        }

        if (closeMenu) {
            this.close(instance.selectId);
        }
    }

    _getEnabledOptionButtons(instance) {
        return Array.from(instance.menu.querySelectorAll('.ux-select-option')).filter((button) => !button.disabled);
    }

    _setHighlightedIndex(instance, targetIndex, scrollIntoView) {
        const targetButton = instance.menu.querySelector(`.ux-select-option[data-index="${targetIndex}"]`);
        if (!targetButton || targetButton.disabled) return;

        instance.menu.querySelectorAll('.ux-select-option.highlighted').forEach((button) => {
            button.classList.remove('highlighted');
        });

        targetButton.classList.add('highlighted');
        instance.highlightedIndex = targetIndex;

        if (scrollIntoView) {
            targetButton.scrollIntoView({ block: 'nearest' });
        }
    }

    _moveHighlighted(instance, direction) {
        const enabledButtons = this._getEnabledOptionButtons(instance);
        if (enabledButtons.length === 0) return;

        let enabledIndex = enabledButtons.findIndex((button) => Number(button.dataset.index) === instance.highlightedIndex);
        if (enabledIndex < 0) {
            enabledIndex = 0;
        } else {
            enabledIndex = (enabledIndex + direction + enabledButtons.length) % enabledButtons.length;
        }

        const targetIndex = Number(enabledButtons[enabledIndex].dataset.index);
        this._setHighlightedIndex(instance, targetIndex, true);
    }

    _selectHighlighted(instance) {
        const highlightedButton = instance.menu.querySelector(`.ux-select-option[data-index="${instance.highlightedIndex}"]`);
        if (!highlightedButton || highlightedButton.disabled) return;
        this._setValue(instance, highlightedButton.dataset.value, true);
    }

    _handleTriggerKeydown(instance, event) {
        if (instance.select.disabled) return;

        if (event.key === 'ArrowDown') {
            event.preventDefault();
            if (!instance.open) {
                this.open(instance.selectId);
            }
            this._moveHighlighted(instance, 1);
            return;
        }

        if (event.key === 'ArrowUp') {
            event.preventDefault();
            if (!instance.open) {
                this.open(instance.selectId);
            }
            this._moveHighlighted(instance, -1);
            return;
        }

        if (event.key === 'Enter' || event.key === ' ') {
            event.preventDefault();
            if (!instance.open) {
                this.open(instance.selectId);
            } else {
                this._selectHighlighted(instance);
            }
            return;
        }

        if (event.key === 'Escape') {
            if (instance.open) {
                event.preventDefault();
                this.close(instance.selectId);
            }
            return;
        }

        if (event.key === 'Tab' && instance.open) {
            this.close(instance.selectId);
        }
    }

    _handleDocumentClick(event) {
        this.instances.forEach((instance) => {
            if (!instance.open) return;
            if (!instance.wrapper.contains(event.target)) {
                this.close(instance.selectId);
            }
        });
    }

    _handleDocumentKeydown(event) {
        if (event.key === 'Escape') {
            const anyOpen = Array.from(this.instances.values()).some(i => i.open);
            if (anyOpen) {
                this.closeAll();
                event.stopPropagation();
            }
        }
    }
}
