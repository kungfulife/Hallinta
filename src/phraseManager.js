// phraseManager.js
class PhraseManager {
    constructor() {
        this.hamisPhrases = [
            "Hamis sees you 👁️👁️",
            "Hamis is watching your mod order... 👁️",
            "Hamis approves of your chaos 👁️✨",
            "Hamis whispers: 'More mods...' 👁️👁️",
            "The all-seeing Hamis observes 👁️🔮",
            "Hamis notes your dedication 👁️📝",
            "Hamis's gaze pierces through reality 👁️⚡",
            "Hamis sees all mod configurations 👁️🎮",
            "Hamis knows your deepest mod secrets 👁️🤫",
            "Hamis judges your load order... 👁️⚖️"
        ];

        this.excitementPhrases = [
            "LETS GOOO! 🚀",
            "Time to break reality! ⚡",
            "Mod chaos incoming! 🌪️",
            "Ready for destruction! 💥",
            "Spell combinations locked! 🔮",
            "Wand crafting mode: ACTIVATED! ⚡",
            "Polymorphine levels: STABLE! 🧪",
            "Gods status: ABOUT TO BE ANGRY! ⚡",
            "Maximum chaos achieved! 🎯",
            "Reality.exe has stopped working! 💻"
        ];

        this.readyPhrases = [
            "Ready.",
            "All systems go.",
            "Mods loaded.",
            "Standing by.",
            "Configuration stable.",
            "Ready to launch.",
            "Systems nominal.",
            "Awaiting orders.",
            "Ready for action."
        ];

        this.phraseTimer = null;
        this.isActive = false;
    }

    startRandomPhrases() {
        if (this.isActive) return;
        this.isActive = true;

        const scheduleNext = () => {
            // Random interval between 5-10 minutes (300000-600000 ms)
            const interval = Math.random() * (600000 - 300000) + 300000;

            this.phraseTimer = setTimeout(() => {
                this.showRandomPhrase();
                scheduleNext(); // Schedule the next one
            }, interval);
        };

        scheduleNext();
    }

    stopRandomPhrases() {
        this.isActive = false;
        if (this.phraseTimer) {
            clearTimeout(this.phraseTimer);
            this.phraseTimer = null;
        }
    }

    showRandomPhrase() {
        const statusBar = document.getElementById('status-bar');
        if (!statusBar) return;

        // Don't override error messages
        const currentText = statusBar.textContent.toLowerCase();
        if (currentText.includes('error') || currentText.includes('failed')) {
            return;
        }

        const rand = Math.random();
        let phrase, className;

        if (rand < 0.15) { // 15% chance for Hamis phrases
            phrase = this.hamisPhrases[Math.floor(Math.random() * this.hamisPhrases.length)];
            className = 'hamis-phrase';
        } else if (rand < 0.25) { // 10% chance for excitement phrases
            phrase = this.excitementPhrases[Math.floor(Math.random() * this.excitementPhrases.length)];
            className = 'excitement-phrase';
        } else { // 75% chance for ready phrases
            phrase = this.readyPhrases[Math.floor(Math.random() * this.readyPhrases.length)];
            className = 'ready-phrase';
        }

        // Clear existing classes
        statusBar.className = 'status-bar';
        statusBar.classList.add(className);
        statusBar.textContent = phrase;

        // Remove special styling after 3 seconds for Hamis/excitement phrases
        if (className !== 'ready-phrase') {
            setTimeout(() => {
                statusBar.className = 'status-bar';
            }, 3000);
        }
    }

    // Method to manually trigger a Hamis phrase (for testing)
    showHamisPhrase() {
        const statusBar = document.getElementById('status-bar');
        if (!statusBar) return;

        const phrase = this.hamisPhrases[Math.floor(Math.random() * this.hamisPhrases.length)];
        statusBar.className = 'status-bar hamis-phrase';
        statusBar.textContent = phrase;

        setTimeout(() => {
            statusBar.className = 'status-bar';
        }, 3000);
    }
}

// Export for use in other files
window.PhraseManager = PhraseManager;
