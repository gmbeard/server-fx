(function(undefined) {
    "use strict";
    
    const home = "home";

    class App {

        constructor() {
            this.mainElement = document.querySelector("div.content");

            let self = this;
            window.addEventListener(
                "hashchange", 
                () => self.documentChanged()
            );

            if (window.location.hash === "") {
                window.location.hash = `#${home}`;
            }
            else {
                this.documentChanged();
            }
        }

        setContent(content) {
            this.contentElement = document.createElement("div");
            this.contentElement.innerHTML = content;
            this.contentElement.classList.add("content");
            this.mainElement.appendChild(this.contentElement);
        }

        load() {
            let self = this;
            fetch(`/content/${window.location.hash.replace("#", "")}`)
                .then(body => body.text())
                .then(text => {
                    self.setContent(text);
                });
        }

        documentChanged() {
            if (window.location.hash === "") {
                return;
            }

            let self = this;
            if (this.contentElement) {
                this.contentElement.classList.add("out-content");
                this.contentElement.addEventListener(
                    "animationend", 
                    () => {
                        self.contentElement.remove();
                        self.load()
                    },
                    { once: true }
                );
            }
            else {
                self.load();
            }
        }
    }

    const app = new App();

})();

