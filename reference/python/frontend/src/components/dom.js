(function (root) {
    "use strict";

    const query = (selector, scope = root.document) => {
        return scope?.querySelector?.(selector) || null;
    };

    const queryAll = (selector, scope = root.document) => {
        return Array.from(scope?.querySelectorAll?.(selector) || []);
    };

    const addClass = (element, className) => {
        element?.classList?.add(className);
    };

    const removeClass = (element, className) => {
        element?.classList?.remove(className);
    };

    const toggleClass = (element, className, force) => {
        if (!element?.classList) {
            return false;
        }
        return element.classList.toggle(className, force);
    };

    const setHidden = (element, hidden) => {
        if (element) {
            element.hidden = Boolean(hidden);
        }
    };

    const setText = (element, value) => {
        if (element) {
            element.textContent = value;
        }
    };

    root.AgentControlDom = {
        query,
        queryAll,
        addClass,
        removeClass,
        toggleClass,
        setHidden,
        setText,
    };
})(typeof window !== "undefined" ? window : globalThis);
