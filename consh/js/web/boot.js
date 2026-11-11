// Load the scripts of the page (index.html) again each time, so that the cache never gives old ones after
// consh/js/build.sh. A file of its own, not a script in the page, so that the Content-Security-Policy that
// serve.py and consh.js set when js is off (script-src 'self') allows it
for (const name of ["conma.js", "serverfs.js", "consh-core.js", "consh.js"]) document.write('<script src="' + name + "?" + Date.now() + '"><\/script>');
