(function ()
{
    if (window.__ytLiteInjected)
    {
        return;
    }
    window.__ytLiteInjected = true;

    var DEFAULT_SPEED = 2;

    function invoke(cmd, args)
    {
        try
        {
            return window.__TAURI_INTERNALS__.invoke(cmd, args);
        }
        catch (e)
        {
            return null;
        }
    }

    // ---------- CSS (lite.css, injected by Rust as window.__YT_LITE_CSS__) ----------
    function addStyle()
    {
        if (document.getElementById('yt-lite-style'))
        {
            return true;
        }
        var root = document.head || document.documentElement;
        if (!root)
        {
            return false;
        }
        var style = document.createElement('style');
        style.id = 'yt-lite-style';
        style.textContent = window.__YT_LITE_CSS__ || '';
        root.appendChild(style);
        return true;
    }
    if (!addStyle())
    {
        var styleObserver = new MutationObserver(function ()
        {
            if (addStyle())
            {
                styleObserver.disconnect();
            }
        });
        styleObserver.observe(document, { childList: true, subtree: true });
    }

    // ---------- Shorts: always open as a normal video ----------
    function shortsToWatch(url)
    {
        var match = url.pathname.match(/^\/shorts\/([\w-]+)/);
        return match ? '/watch?v=' + match[1] : null;
    }

    function redirectIfShorts()
    {
        var target = shortsToWatch(location);
        if (target)
        {
            location.replace(target);
        }
    }
    redirectIfShorts();
    document.addEventListener('yt-navigate-start', redirectIfShorts);
    document.addEventListener('yt-navigate-finish', redirectIfShorts);

    // ---------- Collapsed side menu: YouTube has no History entry there, so add one ----------
    var HISTORY_URL = '/feed/history';
    var HISTORY_ICON = 'M13 3a9 9 0 0 0-9 9H1l3.89 3.89.07.14L9 12H6c0-3.87 3.13-7 7-7s7 3.13 7 7-3.13 7-7 7c-1.93 0-3.68-.79-4.94-2.06l-1.42 1.42A8.954 8.954 0 0 0 13 21a9 9 0 0 0 0-18zm-1 5v5l4.28 2.54.72-1.21-3.5-2.08V8H12z';

    function ensureMiniHistory()
    {
        var subscriptions = document.querySelector('ytd-mini-guide-renderer a[href="/feed/subscriptions"]');
        var subscriptionsEntry = subscriptions && subscriptions.closest('ytd-mini-guide-entry-renderer');
        if (!subscriptionsEntry)
        {
            return;
        }
        var existing = document.getElementById('yt-lite-mini-history');
        if (existing)
        {
            // Follow theme switches (Aparência: claro/escuro)
            var current = getComputedStyle(subscriptions).color;
            if (existing.style.color !== current)
            {
                existing.style.color = current;
                existing.querySelector('svg').style.fill = current;
            }
            return;
        }
        // Page CSS does not reach this spot of the mini guide, so style inline using the neighbour's color
        var color = getComputedStyle(subscriptions).color;
        var link = document.createElement('a');
        link.id = 'yt-lite-mini-history';
        link.href = HISTORY_URL;
        link.title = 'Histórico';
        link.style.cssText = 'display:flex;flex-direction:column;align-items:center;width:64px;padding:16px 0 14px;'
            + 'border-radius:10px;text-decoration:none;font-family:Roboto,Arial,sans-serif;font-size:10px;line-height:16px;color:' + color;
        link.addEventListener('mouseenter', function ()
        {
            link.style.background = 'rgba(127, 127, 127, 0.15)';
        });
        link.addEventListener('mouseleave', function ()
        {
            link.style.background = '';
        });
        var guideIcon = document.querySelector('ytd-guide-renderer a#endpoint[href="' + HISTORY_URL + '"] svg');
        var icon = guideIcon ? guideIcon.cloneNode(true) : svgElement('svg', { viewBox: '0 0 24 24' });
        if (!guideIcon)
        {
            icon.appendChild(svgElement('path', { d: HISTORY_ICON }));
        }
        icon.setAttribute('width', '24');
        icon.setAttribute('height', '24');
        icon.style.cssText = 'width:24px;height:24px;margin-bottom:6px;fill:' + color;
        var text = document.createElement('span');
        text.textContent = 'Histórico';
        link.appendChild(icon);
        link.appendChild(text);
        link.addEventListener('click', function (event)
        {
            event.preventDefault();
            event.stopPropagation();
            // Clicking YouTube's own (hidden) guide entry keeps the in-app navigation without a full reload
            var guideLink = document.querySelector('ytd-guide-renderer a#endpoint[href="' + HISTORY_URL + '"]');
            if (guideLink)
            {
                guideLink.click();
            }
            else
            {
                location.assign(HISTORY_URL);
            }
        });
        subscriptionsEntry.parentNode.insertBefore(link, subscriptionsEntry.nextSibling);
    }
    setInterval(ensureMiniHistory, 1500);

    // ---------- Links ----------
    function isYouTube(url)
    {
        return /(^|\.)youtube\.com$/.test(url.hostname) || url.hostname === 'youtu.be';
    }

    // Google sign-in must stay inside the app so the session cookies land in this webview
    function isGoogleSignIn(url)
    {
        return /^accounts\.google\.com(\.br)?$/.test(url.hostname);
    }

    document.addEventListener('click', function (event)
    {
        var anchor = event.target && event.target.closest ? event.target.closest('a[href]') : null;
        if (!anchor)
        {
            return;
        }
        var url;
        try
        {
            url = new URL(anchor.href, location.href);
        }
        catch (e)
        {
            return;
        }
        if (url.protocol !== 'http:' && url.protocol !== 'https:')
        {
            return;
        }

        var watch = isYouTube(url) ? shortsToWatch(url) : null;
        if (watch)
        {
            event.preventDefault();
            event.stopPropagation();
            location.assign(watch);
            return;
        }

        if (isYouTube(url))
        {
            // Description links go through youtube.com/redirect?q=<real url>
            if (url.pathname === '/redirect' && url.searchParams.get('q'))
            {
                event.preventDefault();
                event.stopPropagation();
                invoke('open_external', { url: url.searchParams.get('q') });
            }
            else if (anchor.target === '_blank')
            {
                event.preventDefault();
                event.stopPropagation();
                location.assign(url.href);
            }
            return;
        }

        if (isGoogleSignIn(url))
        {
            if (anchor.target === '_blank')
            {
                event.preventDefault();
                event.stopPropagation();
                location.assign(url.href);
            }
            return;
        }

        event.preventDefault();
        event.stopPropagation();
        invoke('open_external', { url: url.href });
    }, true);

    var originalOpen = window.open;
    window.open = function (target)
    {
        if (target)
        {
            try
            {
                var url = new URL(String(target), location.href);
                if ((url.protocol === 'http:' || url.protocol === 'https:') && !isYouTube(url) && !isGoogleSignIn(url))
                {
                    invoke('open_external', { url: url.href });
                    return null;
                }
            }
            catch (e)
            {
            }
        }
        return originalOpen.apply(window, arguments);
    };

    // ---------- Every new video starts at 2x (manual changes are kept until the next video) ----------
    var confirmedVideoId = null;

    function applyDefaultSpeed(event)
    {
        var player = document.getElementById('movie_player');
        if (!player || !player.contains(event.target) || !player.getVideoData)
        {
            return;
        }
        var data = player.getVideoData();
        var id = data && data.video_id;
        if (!id || id === confirmedVideoId)
        {
            return;
        }
        if (event.type === 'playing')
        {
            confirmedVideoId = id;
        }
        if (data.isLive || !isFinite(event.target.duration))
        {
            return;
        }
        if (player.setPlaybackRate)
        {
            player.setPlaybackRate(DEFAULT_SPEED);
        }
        else
        {
            event.target.playbackRate = DEFAULT_SPEED;
        }
    }
    document.addEventListener('loadeddata', applyDefaultSpeed, true);
    document.addEventListener('playing', applyDefaultSpeed, true);

    // ---------- Pasted video links: open them instead of searching for them ----------
    // Accepts youtube.com/watch, youtu.be/<id>, /shorts/<id> and /live/<id>; keeps t= and list=
    function videoPathFromText(text)
    {
        var url;
        try
        {
            url = new URL(String(text || '').trim());
        }
        catch (e)
        {
            return null;
        }
        if (!isYouTube(url))
        {
            return null;
        }
        var id = null;
        if (url.hostname === 'youtu.be')
        {
            id = url.pathname.slice(1).split('/')[0];
        }
        else if (url.pathname === '/watch')
        {
            id = url.searchParams.get('v');
        }
        else
        {
            var match = url.pathname.match(/^\/(?:shorts|live|embed)\/([\w-]+)/);
            id = match && match[1];
        }
        if (!id || !/^[\w-]{6,}$/.test(id))
        {
            return null;
        }
        var path = '/watch?v=' + id;
        ['t', 'list', 'index'].forEach(function (name)
        {
            var value = url.searchParams.get(name);
            if (value)
            {
                path += '&' + name + '=' + encodeURIComponent(value);
            }
        });
        return path;
    }

    function searchInput()
    {
        return document.querySelector('input[name="search_query"]');
    }

    function openFromSearchBox(event)
    {
        var input = searchInput();
        var path = input && videoPathFromText(input.value);
        if (!path)
        {
            return;
        }
        event.preventDefault();
        event.stopPropagation();
        input.blur();
        location.assign(path);
    }

    document.addEventListener('keydown', function (event)
    {
        if (event.key === 'Enter' && event.target === searchInput())
        {
            openFromSearchBox(event);
        }
    }, true);

    document.addEventListener('click', function (event)
    {
        var button = event.target && event.target.closest ? event.target.closest('button') : null;
        if (button && button.closest('yt-searchbox, ytd-searchbox') && !button.closest('[class*="TextInputAssistant"]'))
        {
            openFromSearchBox(event);
        }
    }, true);

    document.addEventListener('paste', function (event)
    {
        var target = event.target;
        var editing = target && (target.isContentEditable || /^(INPUT|TEXTAREA)$/.test(target.tagName));
        if (editing || !event.clipboardData)
        {
            return;
        }
        var path = videoPathFromText(event.clipboardData.getData('text'));
        if (path)
        {
            event.preventDefault();
            location.assign(path);
        }
    }, true);

    // ---------- Tabs: Chrome-style shortcuts ----------
    window.addEventListener('keydown', function (event)
    {
        if (!event.ctrlKey || event.altKey)
        {
            return;
        }
        var key = event.key.toLowerCase();
        if (key === 'n')
        {
            invoke('new_tab', { url: null, background: false });
        }
        else if (key === 'w')
        {
            invoke('close_current');
        }
        else if (key === 'tab' || key === 'pagedown' || key === 'pageup')
        {
            invoke('cycle_tab', { offset: (event.shiftKey || key === 'pageup') ? -1 : 1 });
        }
        else
        {
            return;
        }
        event.preventDefault();
        event.stopPropagation();
    }, true);

    // ---------- Tabs opened in the background wait until they are shown before playing ----------
    var waitingForVisible = false;

    document.addEventListener('playing', function (event)
    {
        var player = document.getElementById('movie_player');
        if (document.hidden && player && player.contains(event.target) && event.target.currentTime < 2)
        {
            waitingForVisible = true;
            event.target.pause();
        }
    }, true);

    document.addEventListener('visibilitychange', function ()
    {
        if (!document.hidden && waitingForVisible)
        {
            waitingForVisible = false;
            var video = document.querySelector('#movie_player video');
            if (video)
            {
                video.play();
            }
        }
    });

    // ---------- Mini player: small always-on-top window showing only the video ----------
    // Uses the player's own fullscreen inside a shrunken borderless window, so YouTube sizes the video itself
    var miniMode = false;

    function svgElement(name, attributes)
    {
        var element = document.createElementNS('http://www.w3.org/2000/svg', name);
        Object.keys(attributes).forEach(function (key)
        {
            element.setAttribute(key, attributes[key]);
        });
        return element;
    }

    function enterMini()
    {
        var fullscreenButton = document.querySelector('#movie_player .ytp-fullscreen-button');
        if (!fullscreenButton || miniMode)
        {
            return;
        }
        miniMode = true;
        invoke('set_mini', { on: true });
        if (!document.fullscreenElement)
        {
            fullscreenButton.click();
        }
        else
        {
            addDragBar();
        }
    }

    function exitMini()
    {
        if (!miniMode)
        {
            return;
        }
        miniMode = false;
        var bar = document.getElementById('yt-lite-dragbar');
        if (bar)
        {
            bar.remove();
        }
        invoke('set_mini', { on: false });
        if (document.fullscreenElement)
        {
            document.exitFullscreen();
        }
    }

    function addDragBar()
    {
        if (document.getElementById('yt-lite-dragbar') || !document.fullscreenElement)
        {
            return;
        }
        var host = document.fullscreenElement.contains(document.body) ? document.body : document.fullscreenElement;
        var bar = document.createElement('div');
        bar.id = 'yt-lite-dragbar';
        bar.title = 'Arraste para mover';
        var back = document.createElement('button');
        back.textContent = 'Voltar';
        back.title = 'Sair do mini player';
        back.addEventListener('click', function (event)
        {
            event.stopPropagation();
            exitMini();
        });
        bar.addEventListener('mousedown', function (event)
        {
            if (event.button === 0 && event.target === bar)
            {
                invoke('start_drag');
            }
        });
        bar.appendChild(back);
        host.appendChild(bar);
    }

    function ensureMiniButton()
    {
        var fullscreenButton = document.querySelector('#movie_player .ytp-fullscreen-button');
        if (!fullscreenButton || document.querySelector('#movie_player .yt-lite-mini-button'))
        {
            return;
        }
        // Same shape as YouTube's own control buttons: 24x24 svg inside a .ytp-button
        var label = 'Mini player por cima de tudo';
        var button = document.createElement('button');
        button.className = 'ytp-button yt-lite-mini-button';
        button.setAttribute('aria-label', label);
        button.setAttribute('data-title-no-tooltip', label);
        button.setAttribute('data-tooltip-title', label);
        var icon = svgElement('svg', { viewBox: '0 0 24 24', width: '24', height: '24' });
        icon.appendChild(svgElement('path', {
            fill: '#fff',
            d: 'M19 11h-8v6h8v-6zm4 8V4.98C23 3.88 22.1 3 21 3H3c-1.1 0-2 .88-2 1.98V19c0 1.1.9 2 2 2h18c1.1 0 2-.9 2-2zm-2 .02H3V4.97h18v14.05z'
        }));
        button.appendChild(icon);
        button.addEventListener('click', function (event)
        {
            event.stopPropagation();
            if (miniMode)
            {
                exitMini();
            }
            else
            {
                enterMini();
            }
        });
        fullscreenButton.parentNode.insertBefore(button, fullscreenButton);
    }
    document.addEventListener('loadeddata', ensureMiniButton, true);
    document.addEventListener('yt-navigate-finish', ensureMiniButton);

    // ---------- Player fullscreen button -> real window fullscreen (except in mini mode) ----------
    document.addEventListener('fullscreenchange', function ()
    {
        if (miniMode)
        {
            if (document.fullscreenElement)
            {
                addDragBar();
            }
            else
            {
                // Esc or double-click left the player's fullscreen: leave mini mode too
                exitMini();
            }
            return;
        }
        invoke('set_fullscreen', { fullscreen: !!document.fullscreenElement });
    });
})();
