// Dynamic year in footer
document.getElementById('year').textContent = new Date().getFullYear();

// Smooth scrolling for navigation links
document.querySelectorAll('a[href^="#"]').forEach(anchor => {
    anchor.addEventListener('click', function (e) {
        e.preventDefault();
        const targetId = this.getAttribute('href');
        const targetElement = document.querySelector(targetId);
        
        if (targetElement) {
            window.scrollTo({
                top: targetElement.offsetTop - 80, // Account for fixed navbar
                behavior: 'smooth'
            });
        }
    });
});

// Navbar glass effect on scroll
const navbar = document.querySelector('.navbar');
window.addEventListener('scroll', () => {
    if (window.scrollY > 50) {
        navbar.style.background = 'rgba(255, 255, 255, 0.05)';
        navbar.style.boxShadow = '0 4px 30px rgba(0, 0, 0, 0.1)';
        navbar.style.borderBottom = '1px solid rgba(255, 255, 255, 0.08)';
    } else {
        navbar.style.background = 'transparent';
        navbar.style.boxShadow = 'none';
        navbar.style.borderBottom = 'none';
    }
});

// Interactive hover effect for feature cards
const cards = document.querySelectorAll('.feature-card');
cards.forEach(card => {
    card.addEventListener('mousemove', (e) => {
        const rect = card.getBoundingClientRect();
        const x = e.clientX - rect.left;
        const y = e.clientY - rect.top;
        
        card.style.setProperty('--mouse-x', `${x}px`);
        card.style.setProperty('--mouse-y', `${y}px`);
        
        // Add dynamic glow effect based on mouse position
        card.style.background = `radial-gradient(circle at ${x}px ${y}px, rgba(138, 43, 226, 0.1) 0%, rgba(255, 255, 255, 0.03) 50%)`;
    });
    
    card.addEventListener('mouseleave', () => {
        card.style.background = 'rgba(255, 255, 255, 0.03)';
    });
});

// Reveal elements on scroll
const observerOptions = {
    root: null,
    rootMargin: '0px',
    threshold: 0.1
};

const observer = new IntersectionObserver((entries, observer) => {
    entries.forEach(entry => {
        if (entry.isIntersecting) {
            entry.target.style.opacity = '1';
            entry.target.style.transform = 'translateY(0)';
            observer.unobserve(entry.target);
        }
    });
}, observerOptions);

document.querySelectorAll('.feature-card').forEach((el, index) => {
    el.style.opacity = '0';
    el.style.transform = 'translateY(20px)';
    el.style.transition = `all 0.6s cubic-bezier(0.2, 0.8, 0.2, 1) ${index * 0.1}s`;
    observer.observe(el);
});

// Simple terminal typing effect
const terminalLines = document.querySelectorAll('.term-line');
let currentLine = 0;

function showNextLine() {
    if (currentLine < terminalLines.length) {
        terminalLines[currentLine].style.opacity = '0';
        terminalLines[currentLine].style.transform = 'translateY(5px)';
        terminalLines[currentLine].style.transition = 'all 0.4s ease';
        
        setTimeout(() => {
            terminalLines[currentLine].style.opacity = '1';
            terminalLines[currentLine].style.transform = 'translateY(0)';
            currentLine++;
            setTimeout(showNextLine, 600);
        }, 50);
    }
}

// Start terminal animation when in view
const terminalObserver = new IntersectionObserver((entries) => {
    entries.forEach(entry => {
        if (entry.isIntersecting) {
            showNextLine();
            terminalObserver.unobserve(entry.target);
        }
    });
}, { threshold: 0.5 });

const terminalMock = document.querySelector('.terminal-mock');
if (terminalMock) {
    terminalLines.forEach(line => {
        line.style.opacity = '0';
    });
    terminalObserver.observe(terminalMock);
}
