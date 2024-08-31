cd gh-pages
git reset --hard 16e4f272bae2b861230450d0a27ef83e2e9904a4 # reset to initial commit
cd ..

mdbook build
cp -r book/* gh-pages/

cd gh-pages
git config user.name "Deploy from CI"
git config user.email ""

git add -A
git commit -m 'deploy new book'

git config --unset user.name
git config --unset user.email

git push origin +gh-pages
cd ..
